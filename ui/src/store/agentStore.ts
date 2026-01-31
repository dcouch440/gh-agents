import { create } from 'zustand';
import { api, type Agent } from '../api/client';

interface TierStats {
  total: number;
  available: number;
  max: number;
}

export interface AgentPoolStats {
  orchestrators: TierStats;
  workers: TierStats;
  utilities: TierStats;
}

interface AgentState {
  agents: Agent[];
  stats: AgentPoolStats | null;
  loading: boolean;
  fetch: () => Promise<void>;
}

export const useAgentStore = create<AgentState>((set) => ({
  agents: [],
  stats: null,
  loading: false,
  fetch: async () => {
    set({ loading: true });
    try {
      const result = await api.agents.list();
      // Backend may return { agents, stats } or just Agent[]
      if (Array.isArray(result)) {
        set({ agents: result, loading: false });
      } else {
        const r = result as unknown as { agents: Agent[]; stats: AgentPoolStats };
        set({ agents: r.agents ?? [], stats: r.stats ?? null, loading: false });
      }
    } catch {
      set({ loading: false });
    }
  },
}));
