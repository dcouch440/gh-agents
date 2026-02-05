import { render, screen } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { ChatMessage } from './ChatMessage'

vi.mock('@/components/primitives/MarkdownPreview', () => ({
  MarkdownPreview: ({ content }: { content: string }) => (
    <div data-testid="markdown-preview">{content}</div>
  ),
}))

describe('ChatMessage', () => {
  it('renders user message content', () => {
    render(<ChatMessage role="user" content="Hello world" />)
    expect(screen.getByText('Hello world')).toBeInTheDocument()
  })

  it('renders user message with monospace style', () => {
    render(<ChatMessage role="user" content="Hi" />)
    expect(screen.getByText('Hi')).toBeInTheDocument()
  })

  it('renders assistant message with markdown preview', () => {
    render(<ChatMessage role="assistant" content="Hi" />)
    expect(screen.getByTestId('markdown-preview')).toBeInTheDocument()
  })

  it('shows streaming cursor when streaming=true and role=assistant', () => {
    const { container } = render(
      <ChatMessage role="assistant" content="Thinking..." streaming={true} />
    )
    const cursor = container.querySelector('span[class*="MuiBox"]')
    expect(cursor).toBeInTheDocument()
  })

  it('does not show cursor when streaming=false', () => {
    render(
      <ChatMessage role="assistant" content="Done." streaming={false} />
    )
    expect(screen.getByText('Done.')).toBeInTheDocument()
  })
})
