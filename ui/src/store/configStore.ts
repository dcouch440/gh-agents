import { create } from 'zustand';
import { api, type Config } from '../api/client';

interface ConfigState {
  config: Config | null;
  loading: boolean;
  fetch: () => Promise<void>;
  update: (patch: Partial<Config>) => Promise<void>;
}

export const useConfigStore = create<ConfigState>((set) => ({
  config: null,
  loading: false,
  fetch: async () => {
    set({ loading: true });
    try {
      const config = await api.config.get();
      set({ config, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  update: async (patch) => {
    try {
      const config = await api.config.update(patch);
      set({ config });
    } catch {
      // toast handled by caller
    }
  },
}));
