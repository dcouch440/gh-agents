import { createContext, useMemo, type ReactNode } from 'react'
import { useTheme } from '@mui/material/styles'
import type { TerminalTheme } from './types'
import { deriveTerminalTheme } from './defaultTheme'

const TerminalThemeContext = createContext<TerminalTheme | null>(null)

type TerminalThemeProviderProps = {
  children: ReactNode
}

function TerminalThemeProvider({ children }: TerminalThemeProviderProps) {
  const muiTheme = useTheme()
  const terminalTheme = useMemo(() => deriveTerminalTheme(muiTheme), [muiTheme])

  return (
    <TerminalThemeContext.Provider value={terminalTheme}>
      {children}
    </TerminalThemeContext.Provider>
  )
}

export { TerminalThemeProvider, TerminalThemeContext }
