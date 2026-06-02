import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/react';
import { MarkdownEditor } from './MarkdownEditor';

const TASK_LIST = `Some intro

- [ ] first task
- [x] done task
- [ ] another task`;

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
    // Simulate click — the widget flips [ ] → [x] and onChange fires.
    fireEvent.click(firstCheckbox);
    // onChange is called with the new markdown
    expect(onChange).toHaveBeenCalled();
    const last = onChange.mock.calls[onChange.mock.calls.length - 1]?.[0];
    expect(last).toContain('- [x] first task');
  });
});
