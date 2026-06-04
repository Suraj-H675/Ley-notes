import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, fireEvent, act } from '@testing-library/react';
import 'fake-indexeddb/auto';
import { MarkdownEditor } from './MarkdownEditor';

const MULTI = `cursor here

this is **bold** text`;

const MULTI_EM = `cursor here

an *italic* word`;

const MULTI_CODE = `cursor here

use \`npm install\` here`;

const MULTI_WIKI = `cursor here

see [[Other Note]] for context`;

const MULTI_LINK = `cursor here

see [docs](https://example.com)`;

const MULTI_STRIKE = `cursor here

old ~~deleted~~ new`;

const TASK_LIST = `Some intro

- [ ] first task
- [x] done task
- [ ] another task`;

describe('MarkdownEditor', () => {
  it('renders the initial markdown content in the editor', () => {
    const { container } = render(
      <MarkdownEditor content="hello world" onChange={() => {}} />
    );
    expect(container.textContent).toContain('hello');
  });

  it('calls onChange when the user edits the content', async () => {
    const onChange = vi.fn();
    const { container } = render(
      <MarkdownEditor content="initial" onChange={onChange} />
    );
    const editable = container.querySelector('[contenteditable="true"]');
    expect(editable).toBeTruthy();
    await act(async () => {
      if (editable) {
        editable.textContent = 'updated text';
        editable.dispatchEvent(new Event('input', { bubbles: true }));
      }
    });
    expect(onChange).toHaveBeenCalled();
  });

  it('renders bold text as a styled span on a non-cursor line', () => {
    const { container } = render(
      <MarkdownEditor content={MULTI} onChange={() => {}} />
    );
    const strong = container.querySelector('[data-inline="strong"]');
    expect(strong).toBeTruthy();
    expect(strong?.textContent).toBe('bold');
  });

  it('renders italic text as a styled span', () => {
    const { container } = render(
      <MarkdownEditor content={MULTI_EM} onChange={() => {}} />
    );
    const em = container.querySelector('[data-inline="em"]');
    expect(em).toBeTruthy();
    expect(em?.textContent).toBe('italic');
  });

  it('renders inline code as a styled span', () => {
    const { container } = render(
      <MarkdownEditor content={MULTI_CODE} onChange={() => {}} />
    );
    const code = container.querySelector('[data-inline="code"]');
    expect(code).toBeTruthy();
    expect(code?.textContent).toBe('npm install');
  });

  it('renders a wikilink as a styled span', () => {
    const { container } = render(
      <MarkdownEditor content={MULTI_WIKI} onChange={() => {}} />
    );
    const wikilink = container.querySelector('[data-inline="wikilink"]');
    expect(wikilink).toBeTruthy();
    expect(wikilink?.textContent).toBe('Other Note');
  });

  it('renders a link as a styled span', () => {
    const { container } = render(
      <MarkdownEditor content={MULTI_LINK} onChange={() => {}} />
    );
    const link = container.querySelector('[data-inline="link"]');
    expect(link).toBeTruthy();
    expect(link?.textContent).toBe('docs');
  });

  it('renders strikethrough as a styled span', () => {
    const { container } = render(
      <MarkdownEditor content={MULTI_STRIKE} onChange={() => {}} />
    );
    const strike = container.querySelector('[data-inline="strike"]');
    expect(strike).toBeTruthy();
    expect(strike?.textContent).toBe('deleted');
  });

  it('hides decorations on the cursor line so the user can edit raw markdown', () => {
    const SINGLE = 'cursor is here and **bold** is on this line';
    const { container } = render(
      <MarkdownEditor content={SINGLE} onChange={() => {}} />
    );
    const strong = container.querySelector('[data-inline="strong"]');
    expect(strong).toBeNull();
    expect(container.textContent).toContain('**bold**');
  });

  it('shows a placeholder when the editor is empty', () => {
    const { container } = render(
      <MarkdownEditor
        content=""
        placeholder="Write something..."
        onChange={() => {}}
      />
    );
    const placeholder = container.querySelector('.cm-placeholder');
    expect(placeholder).toBeTruthy();
    expect(placeholder?.textContent).toBe('Write something...');
  });

  it('calls onWikilinkNavigate with the title when a wikilink is clicked', () => {
    const onNav = vi.fn();
    const { container } = render(
      <MarkdownEditor
        content={`cursor here

see [[Other Note]] for context`}
        onChange={() => {}}
        onWikilinkNavigate={onNav}
      />
    );
    const link = container.querySelector('[data-inline="wikilink"]');
    expect(link).toBeTruthy();
    fireEvent.click(link!);
    expect(onNav).toHaveBeenCalledWith('Other Note');
  });
});

describe('MarkdownEditor — task list', () => {
  it('renders a checkbox for each task line', () => {
    const { container } = render(
      <MarkdownEditor content={TASK_LIST} onChange={() => {}} />
    );
    const checkboxes = container.querySelectorAll(
      'input[type="checkbox"][data-task-checkbox]'
    );
    expect(checkboxes).toHaveLength(3);
  });

  it('marks the checkbox as checked when the task is done', () => {
    const { container } = render(
      <MarkdownEditor content={TASK_LIST} onChange={() => {}} />
    );
    const checkboxes = container.querySelectorAll(
      'input[type="checkbox"][data-task-checkbox]'
    );
    expect((checkboxes[0] as HTMLInputElement).checked).toBe(false);
    expect((checkboxes[1] as HTMLInputElement).checked).toBe(true);
    expect((checkboxes[2] as HTMLInputElement).checked).toBe(false);
  });

  it('toggles the underlying markdown when a checkbox is clicked', () => {
    const onChange = vi.fn();
    const { container } = render(
      <MarkdownEditor content={TASK_LIST} onChange={onChange} />
    );
    const firstCheckbox = container.querySelector(
      'input[type="checkbox"][data-task-checkbox]'
    ) as HTMLInputElement;
    expect(firstCheckbox).toBeTruthy();
    fireEvent.click(firstCheckbox);
    expect(onChange).toHaveBeenCalled();
    const last = onChange.mock.calls[onChange.mock.calls.length - 1]?.[0];
    expect(last).toContain('- [x] first task');
  });
});

describe('MarkdownEditor — callouts', () => {
  const CALLOUT = `cursor here

> [!warning] Be careful
> Hot stove ahead.`;

  it('renders a callout as a styled card with the type and title', () => {
    const { container } = render(
      <MarkdownEditor content={CALLOUT} onChange={() => {}} />
    );
    const card = container.querySelector('[data-callout]');
    expect(card).toBeTruthy();
    expect(card?.getAttribute('data-callout-type')).toBe('warning');
    expect(card?.textContent).toContain('Be careful');
    expect(card?.textContent).toContain('Hot stove ahead.');
  });

  it('renders the raw callout source on the cursor line, the card elsewhere', () => {
    // Cursor is on line 1; callout is on lines 3-4. So the card should
    // render (not the raw > [!warning] text).
    const { container } = render(
      <MarkdownEditor content={CALLOUT} onChange={() => {}} />
    );
    const card = container.querySelector('[data-callout]');
    expect(card).toBeTruthy();
    expect(container.textContent).not.toContain('> [!warning] Be careful');
  });

  it('handles callouts without a title', () => {
    const md = `para

> [!tip]
> Just a tip.`;
    const { container } = render(
      <MarkdownEditor content={md} onChange={() => {}} />
    );
    const card = container.querySelector('[data-callout-type="tip"]');
    expect(card).toBeTruthy();
    expect(card?.textContent).toContain('Just a tip.');
  });
});

describe('MarkdownEditor — transclusions', () => {
  beforeEach(async () => {
    const { db } = await import('@/lib/db');
    await db.nodes.clear();
  });

  it('renders a transclusion as a styled card on a non-cursor line', async () => {
    const { db } = await import('@/lib/db');
    await db.nodes.put({
      id: 't1',
      type: 'document',
      title: 'Embedded',
      content: 'Embedded body content.',
      plainText: 'Embedded body content.',
      collections: [],
      tags: [],
      properties: {},
      isArchived: 0,
      createdAt: 0,
      updatedAt: 0,
    });

    const md = `cursor here

see ![[Embedded]] now`;
    const { findByTestId } = render(
      <MarkdownEditor content={md} onChange={() => {}} />
    );

    const embed = await findByTestId('transclusion', undefined, { timeout: 2000 });
    expect(embed).toBeTruthy();
    expect(embed.getAttribute('data-transclusion-title')).toBe('Embedded');
  });

  it('falls back to "Note not found" when the target does not exist', async () => {
    const md = `cursor here

see ![[Missing Note]] now`;
    const { findByTestId } = render(
      <MarkdownEditor content={md} onChange={() => {}} />
    );
    const embed = await findByTestId('transclusion', undefined, { timeout: 2000 });
    expect(embed.textContent).toMatch(/not found/i);
  });

  it('hides the embed card on the cursor line, showing the raw ![[...]] source', () => {
    const md = 'inline ![[Embedded]] here';
    const { container } = render(
      <MarkdownEditor content={md} onChange={() => {}} />
    );
    const embed = container.querySelector('[data-transclusion]');
    expect(embed).toBeNull();
    expect(container.textContent).toContain('![[Embedded]]');
  });
});
