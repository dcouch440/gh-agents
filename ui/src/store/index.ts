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
