import { render, screen, waitFor } from '@testing-library/react'
import { WorkflowProvider } from './WorkflowContext'
import { useWorkflowContext } from '@/hooks/useWorkflowContext'
import { mockWorkflow } from '@/test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

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
  const { workflows, loading, error } = useWorkflowContext()
  if (loading) return <div>loading</div>
  if (error) return <div>error: {error}</div>
  return (
    <div>
      {workflows.map((w) => (
        <div key={w.id} data-testid={`workflow-${w.id}`}>
          {w.name}
        </div>
      ))}
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('WorkflowContext', () => {
  describe('WorkflowProvider', () => {
    beforeEach(() => {
      vi.clearAllMocks()
      mockGet.mockResolvedValue([mockWorkflow])
    })

    it('fetches workflows on mount and renders them', async () => {
      render(
        <WorkflowProvider>
          <TestConsumer />
        </WorkflowProvider>,
      )

      expect(screen.getByText('loading')).toBeInTheDocument()

      await waitFor(() => {
        expect(screen.getByTestId('workflow-workflow-001')).toHaveTextContent('Test Workflow')
      })
    })

    it('handles fetch error', async () => {
      mockGet.mockRejectedValue(new Error('Network error'))

      render(
        <WorkflowProvider>
          <TestConsumer />
        </WorkflowProvider>,
      )

      await waitFor(() => {
        expect(screen.getByText('error: Network error')).toBeInTheDocument()
      })
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useWorkflowContext must be used within WorkflowProvider')
      spy.mockRestore()
    })
  })
})
