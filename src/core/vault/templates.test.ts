import { beforeEach, describe, expect, it } from 'vitest';
import { resetDb } from '@/test/helpers';
import { createPage } from './pages';
import { applyTemplate, listVaultTemplates, templateFrontmatter } from './templates';

describe('vault templates', () => {
  beforeEach(async () => resetDb());

  it('discovers templates by configured vault folder and expands note variables', async () => {
    const template = await createPage({
      title: 'Project brief',
      folder: 'templates',
      content: '# {{title}}\n\nCreated {{date:yyyy-MM-dd}}',
      frontmatter: { title: 'Template title', aliases: ['brief'], status: 'draft' },
    });
    await createPage({ title: 'Ordinary note', folder: 'notes' });

    expect((await listVaultTemplates()).map((page) => page.id)).toEqual([template.id]);
    expect(applyTemplate(template.content, { title: 'Apollo', date: new Date('2026-07-13T00:00:00Z') }))
      .toContain('# Apollo\n\nCreated 2026-07-13');
    expect(templateFrontmatter(template)).toEqual({ status: 'draft' });
  });
});
