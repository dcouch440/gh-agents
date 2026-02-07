import { render, screen, waitFor } from '@testing-library/react'
import { ChatProvider } from './ChatContext'
import { useChatContext } from '@/hooks/useChatContext'
import { mockChatMessage } from '@/test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

const { mockGetHistory } = vi.hoisted(() => ({ mockGetHistory: vi.fn() }))

vi.mock('@/api', () => ({
  api: { sessions: { getHistory: mockGetHistory } },
}))

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
      vi.clearAllMocks()
      mockGetHistory.mockResolvedValue([mockChatMessage])
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

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useChatContext must be used within ChatProvider')
      spy.mockRestore()
    })
  })
})
