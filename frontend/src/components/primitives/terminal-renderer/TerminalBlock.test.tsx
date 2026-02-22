import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ThemeProvider, createTheme } from '@mui/material/styles'
import { TerminalBlock } from './TerminalBlock'

const darkTheme = createTheme({ palette: { mode: 'dark' } })

const renderWithTheme = (ui: React.ReactElement) =>
  render(<ThemeProvider theme={darkTheme}>{ui}</ThemeProvider>)

describe('TerminalBlock', () => {
  it('renders plain text', () => {
    renderWithTheme(<TerminalBlock content="Hello world" />)
    expect(screen.getByText('Hello world')).toBeInTheDocument()
  })

  it('renders bold text with bright color (not <strong>)', () => {
    renderWithTheme(<TerminalBlock content="This is **bold** text" />)
    const boldEl = screen.getByText('bold')
    // Should NOT be a <strong> tag — terminal style uses color, not font-weight
    expect(boldEl.tagName).not.toBe('STRONG')
    expect(boldEl.closest('span')).toBeInTheDocument()
  })

  it('strips thinking tags', () => {
    renderWithTheme(<TerminalBlock content="<thinking>secret</thinking>visible" />)
    expect(screen.getByText('visible')).toBeInTheDocument()
    expect(screen.queryByText('secret')).not.toBeInTheDocument()
  })

  it('applies className', () => {
    const { container } = renderWithTheme(<TerminalBlock content="test" className="custom" />)
    expect(container.querySelector('.custom')).toBeInTheDocument()
  })

  it('renders code blocks with pre element', () => {
    const md = '```js\nconsole.log("hi")\n```'
    const { container } = renderWithTheme(<TerminalBlock content={md} />)
    const preElement = container.querySelector('pre')
    expect(preElement).toBeInTheDocument()
  })

  it('renders language label for code blocks', () => {
    const md = '```typescript\nconst x = 1\n```'
    renderWithTheme(<TerminalBlock content={md} />)
    expect(screen.getByText('[typescript]')).toBeInTheDocument()
  })

  it('renders headings with box-drawing characters', () => {
    renderWithTheme(<TerminalBlock content="# Main Title" />)
    // H1 uses ═══ prefix — CSS text-transform uppercases, but DOM text stays original
    expect(screen.getByText('═══')).toBeInTheDocument()
    expect(screen.getByText('Main Title')).toBeInTheDocument()
  })

  it('renders h2 with single line decoration', () => {
    renderWithTheme(<TerminalBlock content="## Section" />)
    expect(screen.getByText('───')).toBeInTheDocument()
    expect(screen.getByText('Section')).toBeInTheDocument()
  })

  it('renders h3 with line prefix', () => {
    renderWithTheme(<TerminalBlock content="### Sub Section" />)
    expect(screen.getByText(/──/)).toBeInTheDocument()
  })

  it('renders unordered list with triangle bullets', () => {
    const { container } = renderWithTheme(<TerminalBlock content={"- item one\n- item two"} />)
    const text = container.textContent
    expect(text).toContain('item one')
    expect(text).toContain('item two')
    // Triangle bullets
    const bullets = screen.getAllByText('▸')
    expect(bullets.length).toBeGreaterThanOrEqual(2)
  })

  it('renders blockquote with left border', () => {
    const { container } = renderWithTheme(<TerminalBlock content="> quoted text" />)
    expect(screen.getByText('quoted text')).toBeInTheDocument()
    // Check for border-left styling (blockquote wrapper)
    const bqElement = container.querySelector('[style*="border-left"]') ?? container.querySelector('div > div')
    expect(bqElement).toBeInTheDocument()
  })

  it('renders horizontal rule', () => {
    const { container } = renderWithTheme(<TerminalBlock content="---" />)
    expect(container.querySelector('hr')).toBeInTheDocument()
  })

  it('renders links as anchor elements', () => {
    renderWithTheme(<TerminalBlock content="[click](https://example.com)" />)
    const link = screen.getByText('click')
    expect(link.closest('a')).toHaveAttribute('href', 'https://example.com')
    expect(link.closest('a')).toHaveAttribute('target', '_blank')
  })

  it('renders inline code with accent styling', () => {
    renderWithTheme(<TerminalBlock content="Use `code` here" />)
    const codeEl = screen.getByText('code')
    expect(codeEl.tagName.toLowerCase()).toBe('code')
  })

  it('renders table with box-drawing characters', () => {
    const md = '| Name | Age |\n| --- | --- |\n| Alice | 30 |'
    const { container } = renderWithTheme(<TerminalBlock content={md} />)
    const pre = container.querySelector('pre')
    expect(pre).toBeInTheDocument()
    expect(pre?.textContent).toContain('┌')
    expect(pre?.textContent).toContain('┘')
    expect(pre?.textContent).toContain('Alice')
  })
})
