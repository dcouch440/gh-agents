import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { ContextNodeContent } from './ContextNodeContent'

vi.mock('@/components/primitives/MarkdownPreview', () => ({
  MarkdownPreview: ({ content }: { content: string }) => <div data-testid="markdown-preview">{content}</div>,
}))

vi.mock('@/components/primitives/CodeEditor', () => ({
  CodeEditor: ({ value, onChange }: { value: string; onChange?: (v: string) => void }) => (
    <pre data-testid="code-editor" onClick={() => onChange?.('edited')}>{value}</pre>
  ),
}))

describe('ContextNodeContent', () => {
  describe('default state', () => {
    it('renders code editor in raw mode by default', () => {
      render(<ContextNodeContent content="Hello world" onChange={vi.fn()} />)
      expect(screen.getByTestId('code-editor')).toBeInTheDocument()
      expect(screen.getByText('Hello world')).toBeInTheDocument()
    })

    it('renders Raw and Md toggle buttons', () => {
      render(<ContextNodeContent content="Some content" onChange={vi.fn()} />)
      expect(screen.getByText('Raw')).toBeInTheDocument()
      expect(screen.getByText('Md')).toBeInTheDocument()
    })
  })

  describe('view mode toggle', () => {
    it('switches to markdown preview when Md is clicked', async () => {
      const user = userEvent.setup()
      render(<ContextNodeContent content="# Title" onChange={vi.fn()} />)

      await user.click(screen.getByText('Md'))

      expect(screen.getByTestId('markdown-preview')).toBeInTheDocument()
      expect(screen.queryByTestId('code-editor')).not.toBeInTheDocument()
    })

    it('switches back to raw when Raw is clicked after Md', async () => {
      const user = userEvent.setup()
      render(<ContextNodeContent content="# Title" onChange={vi.fn()} />)

      await user.click(screen.getByText('Md'))
      expect(screen.getByTestId('markdown-preview')).toBeInTheDocument()

      await user.click(screen.getByText('Raw'))
      expect(screen.getByTestId('code-editor')).toBeInTheDocument()
      expect(screen.queryByTestId('markdown-preview')).not.toBeInTheDocument()
    })
  })

  describe('onChange callback', () => {
    it('calls onChange when content is edited in code editor', () => {
      const onChange = vi.fn()
      render(<ContextNodeContent content="original" onChange={onChange} />)

      screen.getByTestId('code-editor').click()
      expect(onChange).toHaveBeenCalledWith('edited')
    })
  })
})
