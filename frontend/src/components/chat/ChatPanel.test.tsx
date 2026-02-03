import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ChatPanel, type ChatMessageData } from './ChatPanel'

vi.mock('./ChatMessage', () => ({
  ChatMessage: (props: { role: string; content: string; streaming?: boolean }) => (
    <div
      data-testid="chat-message"
      data-role={props.role}
      data-content={props.content}
      data-streaming={props.streaming ?? ''}
    />
  ),
}))

vi.mock('./ChatInput', () => ({
  ChatInput: (props: { onSend: (msg: string) => void; disabled?: boolean }) => (
    <div
      data-testid="chat-input"
      data-disabled={props.disabled ?? ''}
    />
  ),
}))

beforeEach(() => {
  vi.clearAllMocks()
})

describe('ChatPanel', () => {
  const mockOnSend = vi.fn()

  it('renders empty state when no messages', () => {
    render(<ChatPanel messages={[]} onSend={mockOnSend} />)
    expect(screen.getByText('No messages yet')).toBeInTheDocument()
  })

  it('renders messages', () => {
    const messages: ChatMessageData[] = [
      { id: '1', role: 'user', content: 'Hello' },
      { id: '2', role: 'assistant', content: 'Hi there' },
    ]
    render(<ChatPanel messages={messages} onSend={mockOnSend} />)

    const rendered = screen.getAllByTestId('chat-message')
    expect(rendered).toHaveLength(2)
    expect(rendered[0]).toHaveAttribute('data-role', 'user')
    expect(rendered[1]).toHaveAttribute('data-role', 'assistant')
  })

  it('passes streaming to last assistant message', () => {
    const messages: ChatMessageData[] = [
      { id: '1', role: 'user', content: 'Hello' },
      { id: '2', role: 'assistant', content: 'Hi' },
    ]
    render(<ChatPanel messages={messages} onSend={mockOnSend} streaming />)

    const rendered = screen.getAllByTestId('chat-message')
    expect(rendered[0]).toHaveAttribute('data-streaming', '')
    expect(rendered[1]).toHaveAttribute('data-streaming', 'true')
  })

  it('renders ChatInput', () => {
    render(<ChatPanel messages={[]} onSend={mockOnSend} />)
    expect(screen.getByTestId('chat-input')).toBeInTheDocument()
  })
})
