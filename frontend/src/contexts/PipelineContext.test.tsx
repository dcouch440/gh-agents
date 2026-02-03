import { render, screen, waitFor } from '@testing-library/react'
import { PipelineProvider } from './PipelineContext'
import { usePipelineContext } from '@/hooks/usePipelineContext'
import { mockPipeline, mockPipelineRun } from '@/test/fixtures'

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

const { mockPipelinesList, mockRunsList } = vi.hoisted(() => ({
  mockPipelinesList: vi.fn(),
  mockRunsList: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: { pipelines: { list: mockPipelinesList }, pipelineRuns: { list: mockRunsList } },
}))

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
      mockPipelinesList.mockResolvedValue({ items: [mockPipeline] })
      mockRunsList.mockResolvedValue({ items: [mockPipelineRun] })
    })

    it('fetches pipelines and runs on mount', async () => {
      render(
        <PipelineProvider>
          <TestConsumer />
        </PipelineProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('pipeline-count')).toHaveTextContent('1')
      })

      await waitFor(() => {
        expect(screen.getByTestId('run-run-001')).toHaveTextContent('running')
      })
    })

    it('reloads runs on WS pipeline event', async () => {
      mockPipelinesList.mockResolvedValue({ items: [mockPipeline] })
      mockRunsList.mockResolvedValue({ items: [] })

      render(
        <PipelineProvider>
          <TestConsumer />
        </PipelineProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('pipeline-count')).toHaveTextContent('1')
      })

      // Now update mock to return a run and trigger WS event
      mockRunsList.mockResolvedValue({ items: [mockPipelineRun] })

      // Backend sends pipeline update event (partial data, not a PipelineRun)
      wsHandler?.({ run_id: 'run-001', pipeline_id: 'pipeline-001', event: 'stage_started' })

      await waitFor(() => {
        expect(screen.getByTestId('run-run-001')).toHaveTextContent('running')
      })
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('usePipelineContext must be used within PipelineProvider')
      spy.mockRestore()
    })
  })
})
