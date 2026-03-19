import type { Theme } from '@mui/material/styles'
import type { TerminalTheme } from './types'

const deriveTerminalTheme = (muiTheme: Theme): TerminalTheme => {
  const isDark = muiTheme.palette.mode === 'dark'
  const p = muiTheme.palette

  return {
    fontFamily: 'inherit',
    fontSize: '0.8125rem',
    lineHeight: 1.6,

    text: p.text.primary,
    textSecondary: p.text.secondary,
    textDisabled: isDark ? '#6e7681' : '#a39283',
    background: 'transparent',
    divider: p.divider,
    accent: p.primary.main,

    bright: isDark ? '#f0f6fc' : '#2d1b0e',
    dim: isDark ? '#8b949e' : '#8c7a68',
    dimStrike: isDark ? '#484f58' : '#c4b8a8',

    codeBg: isDark ? 'rgba(255,255,255,0.04)' : 'rgba(0,0,0,0.04)',
    codeText: p.primary.main,
    codeBorder: p.divider,

    headingText: isDark ? '#f0f6fc' : '#2d1b0e',
    headingRule: isDark ? '#30363d' : 'rgba(45,27,14,0.15)',

    quoteBorder: isDark ? '#30363d' : 'rgba(45,27,14,0.15)',
    quoteText: p.text.secondary,

    tableBorder: isDark ? '#30363d' : 'rgba(45,27,14,0.15)',
    tableHeaderText: isDark ? '#f0f6fc' : '#2d1b0e',

    linkText: p.primary.main,
    linkUnderline: isDark ? 'rgba(59,130,246,0.4)' : 'rgba(255,150,79,0.4)',

    checkboxChecked: p.success.main,
    checkboxUnchecked: isDark ? '#6e7681' : '#a39283',
  }
}

export { deriveTerminalTheme }
