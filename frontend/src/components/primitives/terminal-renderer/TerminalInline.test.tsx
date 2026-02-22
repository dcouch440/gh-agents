import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ThemeProvider, createTheme } from '@mui/material/styles'
import { TerminalInline } from './TerminalInline'

const darkTheme = createTheme({ palette: { mode: 'dark' } })

const renderWithTheme = (ui: React.ReactElement) =>
  render(<ThemeProvider theme={darkTheme}>{ui}</ThemeProvider>)

describe('TerminalInline', () => {
  it('renders as inline span', () => {
    const { container } = renderWithTheme(<TerminalInline content="hello" />)
    const span = container.querySelector('span')
    expect(span).toBeInTheDocument()
    // No block elements
    expect(container.querySelector('p')).not.toBeInTheDocument()
    expect(container.querySelector('div')).not.toBeInTheDocument()
  })

  it('renders plain text', () => {
    renderWithTheme(<TerminalInline content="hello world" />)
    expect(screen.getByText('hello world')).toBeInTheDocument()
  })

  it('renders inline code with code element', () => {
    renderWithTheme(<TerminalInline content="use `code` here" />)
    const codeEl = screen.getByText('code')
    expect(codeEl.tagName.toLowerCase()).toBe('code')
  })

  it('renders bold with bright color', () => {
    renderWithTheme(<TerminalInline content="**bold** text" />)
    expect(screen.getByText('bold')).toBeInTheDocument()
    // Should not use <strong>
    expect(screen.getByText('bold').tagName).not.toBe('STRONG')
  })

  it('renders links', () => {
    renderWithTheme(<TerminalInline content="[link](https://example.com)" />)
    const link = screen.getByText('link')
    expect(link.closest('a')).toHaveAttribute('href', 'https://example.com')
  })
})
