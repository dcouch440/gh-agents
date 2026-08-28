import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi } from 'vitest'
import { MessageList } from './MessageList'
import type { ChatMessageData } from './MessageList'

vi.mock('@/components/primitives/terminal-renderer', () => ({
  TerminalBlock: ({ content }: { content: string }) => <div data-testid="markdown-preview">{content}</div>,
  TerminalInline: ({ content }: { content: string }) => <span data-testid="inline-markdown">{content}</span>,
}))

const failedTurn: ChatMessageData[] = [
  {
    id: 'msg-1',
    role: 'user',
    content: 'Turn a random node into a dual agent workflow',
    error: 'LLM call failed (round 0): Stream transport error',
  },
]

describe('MessageList', () => {
  it('surfaces a turn that failed with no reply', () => {
    render(<MessageList messages={failedTurn} />)
    expect(screen.getByText(/Stream transport error/)).toBeInTheDocument()
  })

  it('resends the failed message content on retry', async () => {
    const onRetry = vi.fn()
    render(<MessageList messages={failedTurn} onRetry={onRetry} />)

    await userEvent.click(screen.getByRole('button', { name: 'Retry' }))
    expect(onRetry).toHaveBeenCalledWith('Turn a random node into a dual agent workflow')
  })

  it('shows a working indicator while the reply is still empty', () => {
    const messages: ChatMessageData[] = [
      { id: 'msg-1', role: 'user', content: 'Build me a workflow' },
      { id: 'msg-2', role: 'assistant', content: '' },
    ]
    render(<MessageList messages={messages} streaming />)
    expect(screen.getByRole('status', { name: 'Working' })).toBeInTheDocument()
  })

  it('drops the working indicator once tokens arrive', () => {
    const messages: ChatMessageData[] = [
      { id: 'msg-1', role: 'user', content: 'Build me a workflow' },
      { id: 'msg-2', role: 'assistant', content: 'On it' },
    ]
    render(<MessageList messages={messages} streaming />)
    expect(screen.queryByRole('status', { name: 'Working' })).not.toBeInTheDocument()
  })

  it('keeps the working indicator up between tool rounds', () => {
    const messages: ChatMessageData[] = [
      { id: 'msg-1', role: 'user', content: 'Build me a workflow' },
      { id: 'tool-1', role: 'tool', content: 'ls', toolName: 'run_command' },
    ]
    render(<MessageList messages={messages} streaming />)
    expect(screen.getByRole('status', { name: 'Working' })).toBeInTheDocument()
    expect(screen.getByText(/running…/)).toBeInTheDocument()
  })

  it('defers to streamingContent when the caller renders its own stream', () => {
    const messages: ChatMessageData[] = [{ id: 'msg-1', role: 'user', content: 'Hi' }]
    render(<MessageList messages={messages} streaming streamingContent={<div>segments</div>} />)
    expect(screen.queryByRole('status', { name: 'Working' })).not.toBeInTheDocument()
  })

  it('shows no error notice for a healthy message', () => {
    const messages: ChatMessageData[] = [{ id: 'msg-2', role: 'user', content: 'Hi' }]
    render(<MessageList messages={messages} />)
    expect(screen.queryByText('NO RESPONSE')).not.toBeInTheDocument()
  })
})
