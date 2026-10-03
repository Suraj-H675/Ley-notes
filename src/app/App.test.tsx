import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";

const memoryRoot = vi.hoisted(() => ({
  props: null as Record<string, unknown> | null,
}));

vi.mock("@/features/agent-memory/AgentMemoryWorkspace", () => ({
  AgentMemoryWorkspace: (props: Record<string, unknown>) => {
    memoryRoot.props = props;
    return <div data-testid="agent-memory-root" />;
  },
}));

describe("desktop root", () => {
  it("boots directly into focused Agent Memory without a note-vault capability", () => {
    render(<App />);

    expect(screen.getByTestId("agent-memory-root")).toBeVisible();
    expect(memoryRoot.props).toEqual({});
    expect(memoryRoot.props).not.toHaveProperty("noteExport");
  });
});
