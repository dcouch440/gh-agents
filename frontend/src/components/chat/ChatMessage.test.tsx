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

  it('renders with user class', () => {
    const { container } = render(<ChatMessage role="user" content="Hi" />)
    expect(container.querySelector('.chat-message--user')).toBeInTheDocument()
  })

  it('renders with assistant class', () => {
    const { container } = render(<ChatMessage role="assistant" content="Hi" />)
    expect(container.querySelector('.chat-message--assistant')).toBeInTheDocument()
  })

  it('shows streaming cursor when streaming=true and role=assistant', () => {
    const { container } = render(
      <ChatMessage role="assistant" content="Thinking..." streaming={true} />
    )
    expect(container.querySelector('.chat-message__cursor')).toBeInTheDocument()
  })

  it('does not show cursor when streaming=false', () => {
    const { container } = render(
      <ChatMessage role="assistant" content="Done." streaming={false} />
    )
    expect(container.querySelector('.chat-message__cursor')).not.toBeInTheDocument()
  })
})
