import { render, screen, waitFor } from '@testing-library/react'
import { AgentProvider } from './AgentContext'
import { useAgentContext } from '@/hooks/useAgentContext'
import { mockAgent, mockAgentUpdated } from '@/test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

let wsHandler: ((data: unknown) => void) | null = null

vi.mock('@/hooks/useWebSocket', () => ({
  useWebSocket: () => ({
    status: 'connected' as const,
    subscribe: (_channel: string, handler: (data: unknown) => void) => {
      wsHandler = handler
      return () => { wsHandler = null }
    },
  }),
}))

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('@/api', () => ({
  api: { get: mockGet },
}))

vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})

// ── Test consumer ────────────────────────────────────────────────────────────

function TestConsumer() {
  const { agents, loading, error } = useAgentContext()
  if (loading) return <div>loading</div>
  if (error) return <div>error: {error}</div>
  return (
    <div>
      {agents.map((a) => (
        <div key={a.id} data-testid={`agent-${a.id}`}>
          {a.persona_name}:{a.status}
        </div>
      ))}
    </div>
  )
}

// ── Unit: reducer ────────────────────────────────────────────────────────────

describe('AgentContext', () => {
  // We test the reducer indirectly through the provider since the reducer
  // is not exported. Direct reducer tests would require exporting it.
  // The integration tests below cover all action types.

  describe('AgentProvider', () => {
    beforeEach(() => {
      wsHandler = null
      vi.clearAllMocks()
      mockGet.mockResolvedValue({ agents: [mockAgent] })
    })

    it('fetches agents on mount and renders them', async () => {
      render(
        <AgentProvider>
          <TestConsumer />
        </AgentProvider>,
      )

      expect(screen.getByText('loading')).toBeInTheDocument()

      await waitFor(() => {
        expect(screen.getByTestId('agent-agent-001')).toHaveTextContent('TestBot:idle')
      })
    })

    it('updates an agent via WS message', async () => {
      render(
        <AgentProvider>
          <TestConsumer />
        </AgentProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('agent-agent-001')).toBeInTheDocument()
      })

      // Simulate WS message
      wsHandler?.({ agent: mockAgentUpdated })

      await waitFor(() => {
        expect(screen.getByTestId('agent-agent-001')).toHaveTextContent('TestBot:working')
      })
    })

    it('adds a new agent via WS when id is unknown', async () => {
      render(
        <AgentProvider>
          <TestConsumer />
        </AgentProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('agent-agent-001')).toBeInTheDocument()
      })

      const newAgent = { ...mockAgent, id: 'agent-002', persona_name: 'NewBot' }
      wsHandler?.({ agent: newAgent })

      await waitFor(() => {
        expect(screen.getByTestId('agent-agent-002')).toHaveTextContent('NewBot:idle')
      })
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useAgentContext must be used within AgentProvider')
      spy.mockRestore()
    })
  })
})
