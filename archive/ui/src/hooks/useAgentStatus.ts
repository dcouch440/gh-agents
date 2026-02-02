// TODO: Connect to real data via WebSocket
export function useAgentStatus() {
  return {
    workers: { active: 0, total: 6 },
    orchestrators: { active: 0, total: 1 },
  };
}
