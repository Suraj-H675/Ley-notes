const TASK_LINE = /^(\s*[-*+]\s+)\[([ xX])\](\s+.*)?$/;

export interface MarkdownTask {
  index: number;
  line: number;
  text: string;
  checked: boolean;
}

export function extractMarkdownTasks(content: string): MarkdownTask[] {
  const tasks: MarkdownTask[] = [];
  let fence: string | null = null;
  content.split("\n").forEach((line, lineIndex) => {
    const marker = /^\s*(```|~~~)/.exec(line)?.[1] ?? null;
    if (marker) {
      fence = fence === null ? marker : fence === marker ? null : fence;
      return;
    }
    if (fence) return;
    const match = TASK_LINE.exec(line);
    if (!match) return;
    tasks.push({
      index: tasks.length,
      line: lineIndex + 1,
      text: (match[3] ?? "").trim(),
      checked: match[2].toLowerCase() === "x",
    });
  });
  return tasks;
}

export function countMarkdownTasks(content: string): number {
  return extractMarkdownTasks(content).length;
}

export function toggleMarkdownTask(
  content: string,
  taskIndex: number,
  checked: boolean,
): string {
  if (taskIndex < 0) return content;
  const lines = content.split("\n");
  const indexes = taskLineIndexes(content);
  const lineIndex = indexes[taskIndex];
  if (lineIndex === undefined) return content;
  const match = TASK_LINE.exec(lines[lineIndex]);
  if (!match) return content;
  lines[lineIndex] = `${match[1]}[${checked ? "x" : " "}]${match[3] ?? ""}`;
  return lines.join("\n");
}

function taskLineIndexes(content: string): number[] {
  return extractMarkdownTasks(content).map((task) => task.line - 1);
}
