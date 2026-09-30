/**
 * Native Ley application root.
 *
 * Ley is a focused coding-agent continuity control center. The retired note
 * workspace is no longer a prerequisite for launching the desktop app.
 */

import { AgentMemoryWorkspace } from "@/features/agent-memory/AgentMemoryWorkspace";

export function App() {
  return (
    <AgentMemoryWorkspace
      open
      closable={false}
      onClose={() => undefined}
    />
  );
}
