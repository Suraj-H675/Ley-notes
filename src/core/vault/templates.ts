/**
 * Template variable substitution. Supports:
 *   {{date}}   — formatted date (default "yyyy-MM-dd")
 *   {{time}}   — current time HH:mm
 *   {{title}}  — the page title (passed in by caller)
 *   {{uuid}}   — random nanoid (use sparingly)
 *
 * Format tokens follow date-fns (`yyyy-MM-dd`, `yyyy/MM/dd`, etc.) — NOT
 * the uppercase `YYYY-MM-DD` that some tools (moment.js, Obsidian templates
 * docs) use. Date-fns fails silently on unknown tokens.
 *
 * Variable syntax mirrors Obsidian's Templates plugin.
 */

import { format } from 'date-fns';

export type TemplateVars = {
  title?: string;
  date?: Date;
  /** Override the {{date}} format. Default "yyyy-MM-dd". */
  dateFormat?: string;
};

const VAR_RE = /\{\{\s*(date|time|title|uuid)\s*(?::([^}]+))?\}\}/g;

export function applyTemplate(tmpl: string, vars: TemplateVars = {}): string {
  const date = vars.date ?? new Date();
  const dateFmt = vars.dateFormat ?? 'yyyy-MM-dd';

  return tmpl.replace(VAR_RE, (_, name: string, arg?: string) => {
    switch (name) {
      case 'date':
        return format(date, arg ?? dateFmt);
      case 'time':
        return format(date, arg ?? 'HH:mm');
      case 'title':
        return vars.title ?? '';
      case 'uuid':
        return crypto.randomUUID();
      default:
        return `{{${name}}}`;
    }
  });
}