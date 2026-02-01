import { render, screen, waitFor } from '@testing-library/react'
import { ChatProvider } from './ChatContext'
import { useChatContext } from '../hooks/useChatContext'
import { mockChatMessage, mockAssistantMessage } from '../test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

let wsHandler: ((data: unknown) => void) | null = null

vi.mock('../hooks/useWebSocket', () => ({
  useWebSocket: () => ({
    status: 'connected' as const,
    subscribe: (_channel: string, handler: (data: unknown) => void) => {
      wsHandler = handler
      return () => { wsHandler = null }
    },
  }),
}))

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('../api', () => ({
  api: { get: mockGet },
}))

vi.mock('../constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('../constants')
  return { ...actual, USE_MOCK_DATA: false }
})

// ── Test consumer ────────────────────────────────────────────────────────────

function TestConsumer() {
  const { messages, loading, error } = useChatContext()
  if (loading) return <div>loading</div>
  if (error) return <div>error: {error}</div>
  return (
    <div>
      {messages.map((m) => (
        <div key={m.id} data-testid={`msg-${m.id}`}>
          {m.role}:{m.content}
        </div>
      ))}
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('ChatContext', () => {
  describe('ChatProvider', () => {
    beforeEach(() => {
      wsHandler = null
      vi.clearAllMocks()
      mockGet.mockResolvedValue([mockChatMessage])
    })

    it('fetches chat history for session on mount', async () => {
      render(
        <ChatProvider sessionId="session-001">
          <TestConsumer />
        </ChatProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('msg-msg-001')).toHaveTextContent('user:Hello agent')
      })
    })

    it('appends messages via WS for matching session', async () => {
      render(
        <ChatProvider sessionId="session-001">
          <TestConsumer />
        </ChatProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('msg-msg-001')).toBeInTheDocument()
      })

      wsHandler?.({ session_id: 'session-001', message: mockAssistantMessage })

      await waitFor(() => {
        expect(screen.getByTestId('msg-msg-002')).toHaveTextContent('assistant:Hello human')
      })
    })

    it('ignores WS messages for other sessions', async () => {
      render(
        <ChatProvider sessionId="session-001">
          <TestConsumer />
        </ChatProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('msg-msg-001')).toBeInTheDocument()
      })

      wsHandler?.({ session_id: 'session-999', message: mockAssistantMessage })

      // Should still only have the original message
      expect(screen.queryByTestId('msg-msg-002')).not.toBeInTheDocument()
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useChatContext must be used within ChatProvider')
      spy.mockRestore()
    })
  })
})
