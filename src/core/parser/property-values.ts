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

export function propertyValueError(value: string, original: unknown): string | null {
  if (Array.isArray(original)) {
    const items = value.split(',');
    const emptyItem = items.find((item, index) => item.trim() === '' && index !== items.length - 1);
    return emptyItem !== undefined ? 'Separate list values with commas.' : null;
  }
  if (typeof original === 'boolean' && !/^\s*(true|false)\s*$/i.test(value)) {
    return 'Boolean values must be true or false.';
  }
  if (typeof original === 'number') {
    const trimmed = value.trim();
    if (!trimmed || !Number.isFinite(Number(trimmed))) {
      return 'Numeric values must contain a finite number.';
    }
  }
  if (typeof original === 'object' && original !== null) {
    try {
      JSON.parse(value);
      return null;
    } catch (error) {
      return `Object values must be valid JSON: ${error instanceof Error ? error.message : String(error)}`;
    }
  }
  return null;
}

export function comparePropertyValues(left: unknown, right: unknown): number {
  if (typeof left === 'number' && typeof right === 'number') return left - right;
  if (typeof left === 'boolean' && typeof right === 'boolean') return Number(left) - Number(right);
  return formatPropertyValue(left).localeCompare(formatPropertyValue(right), undefined, {
    numeric: true,
    sensitivity: 'base',
  });
}
