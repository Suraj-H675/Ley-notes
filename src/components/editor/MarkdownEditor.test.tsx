import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent, act } from '@testing-library/react';
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
