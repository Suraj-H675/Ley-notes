export function formatPropertyValue(value: unknown): string {
  if (Array.isArray(value)) return value.map((item) => formatPropertyValue(item)).join(', ');
  if (value === null || value === undefined) return '';
  if (typeof value === 'object') return JSON.stringify(value);
  return String(value);
}

export function parsePropertyValue(value: string, original: unknown): unknown {
  if (Array.isArray(original)) {
    return value
      .split(',')
      .map((item) => item.trim())
      .filter(Boolean);
  }
  if (typeof original === 'boolean') return value.trim().toLowerCase() === 'true';
  if (typeof original === 'number' && value.trim() && Number.isFinite(Number(value))) {
    return Number(value);
  }
  return value;
}

export function comparePropertyValues(left: unknown, right: unknown): number {
  if (typeof left === 'number' && typeof right === 'number') return left - right;
  if (typeof left === 'boolean' && typeof right === 'boolean') return Number(left) - Number(right);
  return formatPropertyValue(left).localeCompare(formatPropertyValue(right), undefined, {
    numeric: true,
    sensitivity: 'base',
  });
}
