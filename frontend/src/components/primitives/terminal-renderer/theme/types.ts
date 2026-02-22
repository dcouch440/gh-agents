type TerminalTheme = {
  fontFamily: string
  fontSize: string
  lineHeight: number

  // Base colors
  text: string
  textSecondary: string
  textDisabled: string
  background: string
  divider: string
  accent: string

  // ANSI emphasis
  bright: string
  dim: string
  dimStrike: string

  // Code
  codeBg: string
  codeText: string
  codeBorder: string

  // Headings
  headingText: string
  headingRule: string

  // Blockquote
  quoteBorder: string
  quoteText: string

  // Table
  tableBorder: string
  tableHeaderText: string

  // Link
  linkText: string
  linkUnderline: string

  // Task list
  checkboxChecked: string
  checkboxUnchecked: string
}

export type { TerminalTheme }
