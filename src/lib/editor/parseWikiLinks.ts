export interface ParsedWikiLink {
  raw: string;
  title: string;
  startIndex: number;
  endIndex: number;
}

const WIKI_LINK_REGEX = /\[\[([^\]]+)\]\]/g;

export function parseWikiLinks(text: string): ParsedWikiLink[] {
  const links: ParsedWikiLink[] = [];
  let match: RegExpExecArray | null;

  while ((match = WIKI_LINK_REGEX.exec(text)) !== null) {
    links.push({
      raw: match[0],
      title: match[1].trim(),
      startIndex: match.index,
      endIndex: match.index + match[0].length,
    });
  }

  WIKI_LINK_REGEX.lastIndex = 0;
  return links;
}

export function extractWikiLinkTitles(text: string): string[] {
  return parseWikiLinks(text).map((link) => link.title);
}

export function replaceWikiLinks(
  text: string,
  replacementFn: (title: string) => string
): string {
  return text.replace(WIKI_LINK_REGEX, (_, title) => {
    return replacementFn(title.trim());
  });
}

export function removeWikiLinks(text: string): string {
  return text.replace(WIKI_LINK_REGEX, '$1');
}

export function hasWikiLinks(text: string): boolean {
  return WIKI_LINK_REGEX.test(text);
}

export function createWikiLink(title: string): string {
  return `[[${title}]]`;
}
