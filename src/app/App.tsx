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
