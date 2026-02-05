import { createContext, useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { LS_THEME } from '@/constants';

type ThemeModeState = {
  mode: 'light' | 'dark';
  toggleMode: () => void;
  setMode: (mode: 'light' | 'dark') => void;
};

const ThemeModeContext = createContext<ThemeModeState | null>(null);

const getSystemPreference = (): 'light' | 'dark' =>
  window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';

const getInitialMode = (): 'light' | 'dark' => {
  const stored = localStorage.getItem(LS_THEME);
  if (stored === 'light' || stored === 'dark') return stored;
  return getSystemPreference();
};

function ThemeModeProvider({ children }: { children: ReactNode }) {
  const [mode, setModeState] = useState<'light' | 'dark'>(getInitialMode);

  const setMode = useCallback((newMode: 'light' | 'dark') => {
    setModeState(newMode);
    localStorage.setItem(LS_THEME, newMode);
  }, []);

  const toggleMode = useCallback(() => {
    setMode(mode === 'light' ? 'dark' : 'light');
  }, [mode, setMode]);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = (e: MediaQueryListEvent) => {
      const stored = localStorage.getItem(LS_THEME);
      if (!stored) {
        setModeState(e.matches ? 'dark' : 'light');
      }
    };
    mediaQuery.addEventListener('change', handler);
    return () => mediaQuery.removeEventListener('change', handler);
  }, []);

  const value = useMemo(() => ({ mode, toggleMode, setMode }), [mode, toggleMode, setMode]);

  return (
    <ThemeModeContext.Provider value={value}>
      {children}
    </ThemeModeContext.Provider>
  );
}

export { ThemeModeContext, ThemeModeProvider };
