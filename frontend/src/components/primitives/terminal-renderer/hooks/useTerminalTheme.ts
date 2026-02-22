import { useContext } from 'react'
import { TerminalThemeContext } from '../theme/TerminalThemeProvider'
import type { TerminalTheme } from '../theme/types'

const useTerminalTheme = (): TerminalTheme => {
  const ctx = useContext(TerminalThemeContext)
  if (ctx === null) {
    throw new Error('useTerminalTheme must be used within a TerminalThemeProvider')
  }
  return ctx
}

export { useTerminalTheme }
