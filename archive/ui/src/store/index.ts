import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface AuthState {
  token: string | null;
  tokenExpiry: number | null;
  isAuthenticated: boolean;
  setToken: (token: string | null, expiresIn?: number) => void;
  logout: () => void;
  isTokenExpired: () => boolean;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      token: null,
      tokenExpiry: null,
      isAuthenticated: false,
      setToken: (token, expiresIn) => set({
        token,
        tokenExpiry: expiresIn ? Date.now() + expiresIn * 1000 : null,
        isAuthenticated: !!token,
      }),
      logout: () => set({ token: null, tokenExpiry: null, isAuthenticated: false }),
      isTokenExpired: () => {
        const { tokenExpiry } = get();
        return tokenExpiry ? Date.now() > tokenExpiry : true;
      },
    }),
    {
      name: 'nexor-auth',
    }
  )
);

interface AppState {
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  sidebarCollapsed: false,
  toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
}));

export interface Toast {
  id: string;
  message: string;
  type: 'success' | 'error' | 'info';
}

interface ToastState {
  toasts: Toast[];
  addToast: (message: string, type?: Toast['type']) => void;
  removeToast: (id: string) => void;
}

export const useToastStore = create<ToastState>((set) => ({
  toasts: [],
  addToast: (message, type = 'info') =>
    set((state) => ({
      toasts: [...state.toasts, { id: crypto.randomUUID(), message, type }],
    })),
  removeToast: (id) =>
    set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) })),
}));

import { api, type SessionResponse, type ModeInfo } from '../api/client';

interface SessionState {
  sessions: SessionResponse[];
  modes: ModeInfo[];
  loaded: boolean;
  load: () => Promise<void>;
  refresh: () => Promise<void>;
  addSession: (session: SessionResponse) => void;
  updateSession: (id: string, updated: SessionResponse) => void;
  removeSession: (id: string) => void;
}

export const useSessionStore = create<SessionState>((set, get) => ({
  sessions: [],
  modes: [],
  loaded: false,
  load: async () => {
    if (get().loaded) return;
    const [sessions, modes] = await Promise.all([
      api.sessions.list().catch(() => [] as SessionResponse[]),
      api.modes.list().catch(() => [] as ModeInfo[]),
    ]);
    set({ sessions, modes, loaded: true });
  },
  refresh: async () => {
    const sessions = await api.sessions.list().catch(() => [] as SessionResponse[]);
    set({ sessions });
  },
  addSession: (session) =>
    set((state) => ({ sessions: [session, ...state.sessions] })),
  updateSession: (id, updated) =>
    set((state) => ({
      sessions: state.sessions.map((s) => (s.id === id ? updated : s)),
    })),
  removeSession: (id) =>
    set((state) => ({
      sessions: state.sessions.filter((s) => s.id !== id),
    })),
}));
