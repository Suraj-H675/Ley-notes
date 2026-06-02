function hashString(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

export function tagColor(tag: string): string {
  const h = hashString(tag) % 360;
  return `hsl(${h}, 50%, 65%)`;
}

export function collectionColor(name: string): string {
  const h = (hashString(name) + 180) % 360;
  return `hsl(${h}, 50%, 65%)`;
}
