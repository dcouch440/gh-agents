import { create } from 'zustand';
import { api, type Task } from '../api/client';

interface TaskFilter {
  status?: string;
  search?: string;
}

interface TaskState {
  tasks: Task[];
  loading: boolean;
  filter: TaskFilter;
  fetch: () => Promise<void>;
  setFilter: (f: Partial<TaskFilter>) => void;
}

export const useTaskStore = create<TaskState>((set, get) => ({
  tasks: [],
  loading: false,
  filter: {},
  fetch: async () => {
    set({ loading: true });
    try {
      const tasks = await api.tasks.list();
      set({ tasks, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  setFilter: (f) => set({ filter: { ...get().filter, ...f } }),
}));
