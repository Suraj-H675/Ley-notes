import { describe, expect, it } from "vitest";
import {
  countMarkdownTasks,
  extractMarkdownTasks,
  toggleMarkdownTask,
} from "./tasks";

describe("Markdown tasks", () => {
  const source = [
    "- [ ] First",
    "  * [x] Nested",
    "```md",
    "- [ ] Example only",
    "```",
    "+ [ ] Third",
  ].join("\n");

  it("counts source tasks but ignores fenced examples", () => {
    expect(countMarkdownTasks(source)).toBe(3);
  });

  it("toggles the requested task while preserving bullets and indentation", () => {
    expect(toggleMarkdownTask(source, 0, true)).toContain("- [x] First");
    expect(toggleMarkdownTask(source, 1, false)).toContain("  * [ ] Nested");
    expect(toggleMarkdownTask(source, 2, true)).toContain("+ [x] Third");
  });

  it("leaves content unchanged for an invalid task index", () => {
    expect(toggleMarkdownTask(source, 99, true)).toBe(source);
  });

  it("extracts task state, text, and source line while ignoring fenced examples", () => {
    expect(
      extractMarkdownTasks(
        "intro\n- [ ] Call Alice\n* [X] Ship release\n```md\n- [ ] example\n```",
      ),
    ).toEqual([
      { index: 0, line: 2, text: "Call Alice", checked: false },
      { index: 1, line: 3, text: "Ship release", checked: true },
    ]);
  });
});
