/**
 * Daily notes — opens or creates the daily note for the given date.
 *
 * Naming: configurable via the `daily-note-format` setting, which follows
 * date-fns tokens (default "yyyy-MM-dd"). Bodies: populated from the
 * `daily-note-template` setting, with `{{date}}`, `{{time}}`, `{{title}}`
 * substituted.
 *
 * Idempotent — if today's daily note already exists, just opens it.
 */

import { db } from '@/data/db';
import { createPage, getPageByTitle } from './pages';
import { applyTemplate } from './templates';
import { format } from 'date-fns';

export interface DailyNoteResult {
  pageId: string;
  title: string;
  created: boolean;
}

export async function getDailyNoteTitle(date: Date = new Date()): Promise<string> {
  const setting = await db.settings.get('daily-note-format');
  const fmt = (setting?.value as string) ?? 'yyyy-MM-dd';
  return format(date, fmt);
}

export async function getOrCreateDailyNote(
  date: Date = new Date(),
): Promise<DailyNoteResult> {
  const title = await getDailyNoteTitle(date);
  const existing = await getPageByTitle(title);
  if (existing && existing.deletedAt === null) {
    return { pageId: existing.id, title, created: false };
  }

  // Apply template.
  const tmplSetting = await db.settings.get('daily-note-template');
  const tmpl = (tmplSetting?.value as string) ?? `# {{date}}\n\n`;
  const content = applyTemplate(tmpl, { title, date });

  const page = await createPage({ title, content });
  return { pageId: page.id, title, created: true };
}