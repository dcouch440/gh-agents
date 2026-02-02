import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor, act } from '@testing-library/react'
import { renderHook } from '@testing-library/react'
import { useContext, type ReactNode } from 'react'
import { PipelineRunProvider, PipelineRunContext } from './PipelineRunContext'
import { mockExecutionTree } from '@/test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

const mockSubscribeRun = vi.hoisted(() => vi.fn(() => vi.fn()))
const mockUnsubscribeRun = vi.hoisted(() => vi.fn())
const mockGet = vi.hoisted(() => vi.fn())

vi.mock('@/hooks/useWebSocket', () => ({
  useWebSocket: () => ({
    status: 'connected' as const,
    subscribe: vi.fn(() => vi.fn()),
    subscribeRun: mockSubscribeRun,
    unsubscribeRun: mockUnsubscribeRun,
    send: vi.fn(),
  }),
}))

vi.mock('@/api', () => ({
  api: {
    get: mockGet,
    post: vi.fn(),
    patch: vi.fn(),
    put: vi.fn(),
    del: vi.fn(),
  },
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

function TestConsumer() {
  const ctx = useContext(PipelineRunContext)
  if (!ctx) return <div>no context</div>
  return (
    <div>
      <span data-testid="loading">{String(ctx.loading)}</span>
      <span data-testid="status">{ctx.tree?.run.status ?? 'none'}</span>
      <span data-testid="error">{ctx.error ?? 'none'}</span>
      {ctx.tree?.stages[0]?.stage_executions[0]?.agent_executions.map((ae) => (
        <span key={ae.id} data-testid={`ae-${ae.id}`}>{ae.status}</span>
      ))}
    </div>
  )
}

const wrapper = ({ children }: { children: ReactNode }) => (
  <PipelineRunProvider runId="run-001">{children}</PipelineRunProvider>
)

// ── Tests ────────────────────────────────────────────────────────────────────

describe('PipelineRunContext', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('fetches tree on mount and renders run status', async () => {
    mockGet.mockResolvedValueOnce(mockExecutionTree)

    render(
      <PipelineRunProvider runId="run-001">
        <TestConsumer />
      </PipelineRunProvider>,
    )

    expect(screen.getByTestId('loading').textContent).toBe('true')

    await waitFor(() => {
      expect(screen.getByTestId('status').textContent).toBe('running')
    })

    expect(screen.getByTestId('loading').textContent).toBe('false')
  })

  it('updates agent execution via WS event', async () => {
    mockGet.mockResolvedValueOnce(mockExecutionTree)

    render(
      <PipelineRunProvider runId="run-001">
        <TestConsumer />
      </PipelineRunProvider>,
    )

    await waitFor(() => {
      expect(screen.getByTestId('status').textContent).toBe('running')
    })

    // Find the agent_execution_update handler
    const agentExecCall = mockSubscribeRun.mock.calls.find(
      (c: unknown[]) => c[1] === 'agent_execution_update',
    )
    expect(agentExecCall).toBeTruthy()
    const handler = agentExecCall![2] as (data: unknown) => void

    act(() => {
      handler({
        run_id: 'run-001',
        agent_execution_id: 'agent-exec-001',
        status: 'completed',
        output: 'done',
        structured_output: null,
        input_tokens: 500,
        output_tokens: 200,
        completed_at: '2025-01-01T00:01:00Z',
      })
    })

    await waitFor(() => {
      expect(screen.getByTestId('ae-agent-exec-001').textContent).toBe('completed')
    })
  })

  it('sets error on fetch failure', async () => {
    mockGet.mockRejectedValueOnce(new Error('Network error'))

    render(
      <PipelineRunProvider runId="run-001">
        <TestConsumer />
      </PipelineRunProvider>,
    )

    await waitFor(() => {
      expect(screen.getByTestId('error').textContent).toBe('Network error')
    })
  })

  it('throws when used outside provider', () => {
    const { result } = renderHook(() => useContext(PipelineRunContext))
    expect(result.current).toBeNull()
  })

  it('calls unsubscribeRun on cleanup', async () => {
    mockGet.mockResolvedValueOnce(mockExecutionTree)

    const { unmount } = render(
      <PipelineRunProvider runId="run-001">
        <TestConsumer />
      </PipelineRunProvider>,
    )

    await waitFor(() => {
      expect(screen.getByTestId('status').textContent).toBe('running')
    })

    unmount()

    expect(mockUnsubscribeRun).toHaveBeenCalledWith('run-001')
  })
})
