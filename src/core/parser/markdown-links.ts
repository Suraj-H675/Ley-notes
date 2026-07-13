export interface InternalMarkdownLink {
  raw: string;
  label: string;
  destination: string;
  path: string;
  heading: string | null;
  blockId: string | null;
  position: number;
  pathFrom: number;
  pathTo: number;
  rootRelative: boolean;
  explicitRelative: boolean;
}

export function extractInternalMarkdownLinks(source: string): InternalMarkdownLink[] {
  const links: InternalMarkdownLink[] = [];
  let cursor = 0;
  let fence: string | null = null;
  for (const line of source.split('\n')) {
    const marker = /^\s*(```|~~~)/.exec(line)?.[1] ?? null;
    if (marker) {
      fence = fence === null ? marker : fence === marker ? null : fence;
      cursor += line.length + 1;
      continue;
    }
    if (!fence) scanLine(line, cursor, links);
    cursor += line.length + 1;
  }
  return links;
}

export function resolveInternalMarkdownPath(sourcePath: string, targetPath: string): string | null {
  if (!targetPath) return normalizeVaultPath(sourcePath);
  const decodedValue = decodePath(targetPath);
  if (decodedValue === null) return null;
  const decoded = decodedValue.replace(/\\([ ()[\]])/g, '$1');
  const rootRelative = decoded.startsWith('/');
  const sourceFolder = sourcePath.includes('/') ? sourcePath.split('/').slice(0, -1) : [];
  const segments = rootRelative ? [] : [...sourceFolder];
  for (const segment of decoded.replace(/^\/+/, '').replace(/\\/g, '/').split('/')) {
    if (!segment || segment === '.') continue;
    if (segment === '..') {
      if (segments.length === 0) return null;
      segments.pop();
    } else {
      segments.push(segment);
    }
  }
  const resolved = segments.join('/');
  return resolved && /\.md$/i.test(resolved) ? resolved : null;
}

export function retargetInternalMarkdownLinks(
  content: string,
  oldSourcePath: string,
  newSourcePath: string,
  pathChanges: ReadonlyMap<string, string>,
): string {
  let output = content;
  const links = extractInternalMarkdownLinks(content).sort((left, right) => right.position - left.position);
  for (const link of links) {
    const oldTarget = resolveInternalMarkdownPath(oldSourcePath, link.path);
    if (!oldTarget) continue;
    const newTarget = pathChanges.get(oldTarget.toLowerCase()) ?? oldTarget;
    const sourceMoved = oldSourcePath.toLowerCase() !== newSourcePath.toLowerCase();
    const targetMoved = oldTarget.toLowerCase() !== newTarget.toLowerCase();
    if (!sourceMoved && !targetMoved) continue;
    const replacement = destinationPath(newSourcePath, newTarget, link);
    output = `${output.slice(0, link.pathFrom)}${replacement}${output.slice(link.pathTo)}`;
  }
  return output;
}

function scanLine(line: string, offset: number, links: InternalMarkdownLink[]): void {
  const visible = maskInlineCode(line);
  for (let index = 0; index < visible.length; index += 1) {
    if (visible[index] !== '[' || visible[index - 1] === '!' || visible[index + 1] === '[') continue;
    const labelEnd = closingDelimiter(visible, index, '[', ']');
    if (labelEnd < 0 || visible[labelEnd + 1] !== '(') continue;
    const destinationEnd = closingDelimiter(visible, labelEnd + 1, '(', ')');
    if (destinationEnd < 0) continue;
    const insideStart = labelEnd + 2;
    const inside = line.slice(insideStart, destinationEnd).trimStart();
    const leading = line.slice(insideStart, destinationEnd).length - inside.length;
    const angle = inside.startsWith('<');
    const rawDestination = angle
      ? inside.slice(1, inside.indexOf('>') >= 0 ? inside.indexOf('>') : 0)
      : destinationToken(inside);
    if (!rawDestination) { index = destinationEnd; continue; }
    const parsed = parseInternalMarkdownDestination(rawDestination);
    if (!parsed) { index = destinationEnd; continue; }
    const destinationStart = offset + insideStart + leading + (angle ? 1 : 0);
    links.push({
      raw: line.slice(index, destinationEnd + 1),
      label: unescapeLabel(line.slice(index + 1, labelEnd)),
      destination: rawDestination,
      path: parsed.path,
      heading: parsed.heading,
      blockId: parsed.blockId,
      position: offset + index,
      pathFrom: destinationStart,
      pathTo: destinationStart + parsed.rawPathLength,
      rootRelative: parsed.path.startsWith('/'),
      explicitRelative: parsed.path.startsWith('./'),
    });
    index = destinationEnd;
  }
}

export function parseInternalMarkdownDestination(destination: string): { path: string; heading: string | null; blockId: string | null; rawPathLength: number } | null {
  if (/^[a-z][a-z\d+.-]*:/i.test(destination) || destination.startsWith('//')) return null;
  const hashAt = destination.indexOf('#');
  const rawPath = hashAt >= 0 ? destination.slice(0, hashAt) : destination;
  if (rawPath && !/\.md$/i.test(rawPath)) return null;
  if (!rawPath && hashAt < 0) return null;
  const rawAnchor = hashAt >= 0 ? destination.slice(hashAt + 1) : '';
  const anchor = decodePath(rawAnchor) ?? rawAnchor;
  return {
    path: rawPath,
    heading: anchor && !anchor.startsWith('^') ? anchor : null,
    blockId: anchor.startsWith('^') ? anchor.slice(1) : null,
    rawPathLength: rawPath.length,
  };
}

function destinationPath(sourcePath: string, targetPath: string, link: InternalMarkdownLink): string {
  if (!link.path && sourcePath.toLowerCase() === targetPath.toLowerCase()) return '';
  if (link.rootRelative) return `/${encodeVaultPath(targetPath)}`;
  const sourceFolder = sourcePath.includes('/') ? sourcePath.split('/').slice(0, -1) : [];
  const target = targetPath.split('/');
  let shared = 0;
  while (shared < sourceFolder.length && shared < target.length && sourceFolder[shared].toLowerCase() === target[shared].toLowerCase()) shared += 1;
  const relative = [...Array(sourceFolder.length - shared).fill('..'), ...target.slice(shared)].join('/') || (target.at(-1) ?? '');
  const encoded = encodeVaultPath(relative);
  return link.explicitRelative && !encoded.startsWith('.') ? `./${encoded}` : encoded;
}

function normalizeVaultPath(path: string): string | null {
  const normalized = path.replace(/\\/g, '/').replace(/^\/+/, '');
  return normalized && !normalized.split('/').some((part) => !part || part === '.' || part === '..') ? normalized : null;
}

function encodeVaultPath(path: string): string {
  return path.split('/').map((part) => part === '..' || part === '.' ? part : encodeURIComponent(part)).join('/');
}

function decodePath(path: string): string | null {
  try { return decodeURIComponent(path); } catch { return null; }
}

function destinationToken(value: string): string {
  let escaped = false;
  for (let index = 0; index < value.length; index += 1) {
    if (!escaped && /\s/.test(value[index])) return value.slice(0, index);
    if (!escaped && value[index] === '\\') escaped = true;
    else escaped = false;
  }
  return value;
}

function closingDelimiter(value: string, start: number, open: string, close: string): number {
  let depth = 0;
  let escaped = false;
  for (let index = start; index < value.length; index += 1) {
    const character = value[index];
    if (!escaped && character === open) depth += 1;
    if (!escaped && character === close && --depth === 0) return index;
    if (!escaped && character === '\\') escaped = true;
    else escaped = false;
  }
  return -1;
}

function maskInlineCode(line: string): string {
  let output = line;
  const expression = /(`+)([\s\S]*?)\1/g;
  output = output.replace(expression, (match) => ' '.repeat(match.length));
  return output;
}

function unescapeLabel(label: string): string {
  return label.replace(/\\([\\[\]])/g, '$1');
}
