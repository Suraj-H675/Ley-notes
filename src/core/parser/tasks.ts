const TASK_LINE = /^(\s*[-*+]\s+)\[([ xX])\](\s+.*)?$/;

export function countMarkdownTasks(content: string): number {
  return taskLineIndexes(content).length;
}

export function toggleMarkdownTask(content: string, taskIndex: number, checked: boolean): string {
  if (taskIndex < 0) return content;
  const lines = content.split('\n');
  const indexes = taskLineIndexes(content);
  const lineIndex = indexes[taskIndex];
  if (lineIndex === undefined) return content;
  const match = TASK_LINE.exec(lines[lineIndex]);
  if (!match) return content;
  lines[lineIndex] = `${match[1]}[${checked ? 'x' : ' '}]${match[3] ?? ''}`;
  return lines.join('\n');
}

function taskLineIndexes(content: string): number[] {
  const indexes: number[] = [];
  let fence: string | null = null;
  content.split('\n').forEach((line, index) => {
    const marker = /^\s*(```|~~~)/.exec(line)?.[1] ?? null;
    if (marker) {
      fence = fence === null ? marker : fence === marker ? null : fence;
      return;
    }
    if (!fence && TASK_LINE.test(line)) indexes.push(index);
  });
  return indexes;
}
