import { createContext, useMemo, type ReactNode } from 'react'
import { useStore } from '@/stores/lib'
import { uiStore } from '@/stores/uiStore'

type ThemeModeState = {
  mode: 'light' | 'dark'
  toggleMode: () => void
  setMode: (mode: 'light' | 'dark') => void
}

const ThemeModeContext = createContext<ThemeModeState | null>(null)

function ThemeModeProvider({ children }: { children: ReactNode }) {
  const mode = useStore(uiStore.store, uiStore.selectTheme)

  const value = useMemo<ThemeModeState>(
    () => ({
      mode,
      toggleMode: uiStore.toggleTheme,
      setMode: uiStore.setTheme,
    }),
    [mode],
  )

  return <ThemeModeContext.Provider value={value}>{children}</ThemeModeContext.Provider>
}

export { ThemeModeContext, ThemeModeProvider }
