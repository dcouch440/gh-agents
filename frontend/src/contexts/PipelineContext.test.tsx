import { render, screen, waitFor } from '@testing-library/react'
import { PipelineProvider } from './PipelineContext'
import { usePipelineContext } from '../hooks/usePipelineContext'
import { mockPipeline, mockPipelineRun } from '../test/fixtures'

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
  const { pipelines, runs, loading, error } = usePipelineContext()
  if (loading) return <div>loading</div>
  if (error) return <div>error: {error}</div>
  return (
    <div>
      <div data-testid="pipeline-count">{pipelines.length}</div>
      {runs.map((r) => (
        <div key={r.id} data-testid={`run-${r.id}`}>
          {r.status}
        </div>
      ))}
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('PipelineContext', () => {
  describe('PipelineProvider', () => {
    beforeEach(() => {
      wsHandler = null
      vi.clearAllMocks()
      mockGet.mockResolvedValue([mockPipeline])
    })

    it('fetches pipelines on mount', async () => {
      render(
        <PipelineProvider>
          <TestConsumer />
        </PipelineProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('pipeline-count')).toHaveTextContent('1')
      })
    })

    it('adds a run via WS', async () => {
      render(
        <PipelineProvider>
          <TestConsumer />
        </PipelineProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('pipeline-count')).toHaveTextContent('1')
      })

      wsHandler?.({ run: mockPipelineRun })

      await waitFor(() => {
        expect(screen.getByTestId('run-run-001')).toHaveTextContent('running')
      })
    })

    it('updates an existing run via WS', async () => {
      render(
        <PipelineProvider>
          <TestConsumer />
        </PipelineProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('pipeline-count')).toBeInTheDocument()
      })

      wsHandler?.({ run: mockPipelineRun })
      await waitFor(() => {
        expect(screen.getByTestId('run-run-001')).toHaveTextContent('running')
      })

      wsHandler?.({ run: { ...mockPipelineRun, status: 'completed' } })
      await waitFor(() => {
        expect(screen.getByTestId('run-run-001')).toHaveTextContent('completed')
      })
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('usePipelineContext must be used within PipelineProvider')
      spy.mockRestore()
    })
  })
})
