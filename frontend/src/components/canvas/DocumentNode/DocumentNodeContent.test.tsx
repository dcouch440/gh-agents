import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { DocumentNodeContent } from './DocumentNodeContent'

// Mock MarkdownPreview to avoid rendering a full markdown engine
vi.mock('@/components/primitives/MarkdownPreview', () => ({
  MarkdownPreview: ({ content }: { content: string }) => <div data-testid="markdown-preview">{content}</div>,
}))

// Mock CodeEditor to avoid Monaco/CodeMirror dependencies
vi.mock('@/components/primitives/CodeEditor', () => ({
  CodeEditor: ({ value }: { value: string }) => <pre data-testid="code-editor">{value}</pre>,
}))

describe('DocumentNodeContent', () => {
  describe('empty state', () => {
    it('renders placeholder when content is empty', () => {
      render(<DocumentNodeContent content="" />)
      expect(screen.getByText('Document will be generated when workflow runs.')).toBeInTheDocument()
    })

    it('renders placeholder when content is only whitespace', () => {
      render(<DocumentNodeContent content="   " />)
      expect(screen.getByText('Document will be generated when workflow runs.')).toBeInTheDocument()
    })

    it('does not render view mode toggle when empty', () => {
      render(<DocumentNodeContent content="" />)
      expect(screen.queryByText('Raw')).not.toBeInTheDocument()
      expect(screen.queryByText('Md')).not.toBeInTheDocument()
    })
  })

  describe('with content', () => {
    it('renders markdown preview by default', () => {
      render(<DocumentNodeContent content="# Hello" />)
      expect(screen.getByTestId('markdown-preview')).toBeInTheDocument()
      expect(screen.getByText('# Hello')).toBeInTheDocument()
    })

    it('renders view mode toggle buttons', () => {
      render(<DocumentNodeContent content="Some content" />)
      expect(screen.getByText('Raw')).toBeInTheDocument()
      expect(screen.getByText('Md')).toBeInTheDocument()
    })

    it('switches to raw view when Raw button is clicked', async () => {
      const user = userEvent.setup()
      render(<DocumentNodeContent content="Hello world" />)

      await user.click(screen.getByText('Raw'))

      expect(screen.getByTestId('code-editor')).toBeInTheDocument()
      expect(screen.getByText('Hello world')).toBeInTheDocument()
    })

    it('switches back to markdown view when Md button is clicked', async () => {
      const user = userEvent.setup()
      render(<DocumentNodeContent content="Hello world" />)

      // Switch to raw
      await user.click(screen.getByText('Raw'))
      expect(screen.getByTestId('code-editor')).toBeInTheDocument()

      // Switch back to markdown
      await user.click(screen.getByText('Md'))
      expect(screen.getByTestId('markdown-preview')).toBeInTheDocument()
    })
  })
})
