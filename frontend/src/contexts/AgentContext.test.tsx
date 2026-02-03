import { render, screen, waitFor } from '@testing-library/react'
import { AgentProvider } from './AgentContext'
import { useAgentContext } from '@/hooks/useAgentContext'
import { mockAgent } from '@/test/fixtures'

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

const { mockList } = vi.hoisted(() => ({ mockList: vi.fn() }))

vi.mock('@/api', () => ({
  api: { agents: { list: mockList } },
}))

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
      mockList.mockResolvedValue({ agents: [mockAgent] })
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

    it('updates an agent status via WS partial update', async () => {
      render(
        <AgentProvider>
          <TestConsumer />
        </AgentProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('agent-agent-001')).toBeInTheDocument()
      })

      // Backend sends partial update: { id, status, current_task, user_id }
      wsHandler?.({ id: 'agent-001', status: 'working', current_task: null })

      await waitFor(() => {
        expect(screen.getByTestId('agent-agent-001')).toHaveTextContent('TestBot:working')
      })
    })

    it('ignores WS update for unknown agent id', async () => {
      render(
        <AgentProvider>
          <TestConsumer />
        </AgentProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('agent-agent-001')).toBeInTheDocument()
      })

      // Unknown agent — should not add a new entry
      wsHandler?.({ id: 'agent-999', status: 'idle', current_task: null })

      await waitFor(() => {
        expect(screen.queryByTestId('agent-agent-999')).not.toBeInTheDocument()
      })
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useAgentContext must be used within AgentProvider')
      spy.mockRestore()
    })
  })
})
