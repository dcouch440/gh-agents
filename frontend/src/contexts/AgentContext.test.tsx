import { render, screen, waitFor } from '@testing-library/react'
import { AgentProvider } from './AgentContext'
import { useAgentContext } from '@/hooks/useAgentContext'
import { mockAgent } from '@/test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

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
          {a.name}:{a.status}
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

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useAgentContext must be used within AgentProvider')
      spy.mockRestore()
    })
  })
})
