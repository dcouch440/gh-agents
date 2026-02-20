import { createContext, useMemo, type ReactNode } from 'react'
import { useStore } from '@/stores/lib'
import { uiStore } from '@/stores/uiStore'
import type { ThemeId } from '@/theme'

type ThemeModeState = {
  themeId: ThemeId
  setTheme: (id: ThemeId) => void
  cycleTheme: () => void
}

const ThemeModeContext = createContext<ThemeModeState | null>(null)

function ThemeModeProvider({ children }: { children: ReactNode }) {
  const themeId = useStore(uiStore.store, uiStore.selectTheme)

  const value = useMemo<ThemeModeState>(
    () => ({
      themeId,
      setTheme: uiStore.setTheme,
      cycleTheme: uiStore.cycleTheme,
    }),
    [themeId],
  )

  return <ThemeModeContext.Provider value={value}>{children}</ThemeModeContext.Provider>
}

export { ThemeModeContext, ThemeModeProvider }
