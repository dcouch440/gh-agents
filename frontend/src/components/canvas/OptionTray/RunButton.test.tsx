import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, act } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { RunButton } from './RunButton'
import { mockWorkflowStep } from '@/test/fixtures'

const {
  mockSelectActiveWorkflowId,
  mockSelectSteps,
  mockRunWorkflow,
  mockCancelWorkflow,
  mockSelectIsRunning,
  mockSelectRunId,
  mockBeginRun,
  mockHydrateActive,
} = vi.hoisted(() => ({
  mockSelectIsRunning: vi.fn<() => boolean>(() => false),
  mockSelectRunId: vi.fn<() => string | null>(() => null),
  mockBeginRun: vi.fn(),
  mockHydrateActive: vi.fn(() => Promise.resolve()),
  mockSelectActiveWorkflowId: vi.fn<() => string | null>(() => 'wf-001'),
  mockSelectSteps: vi.fn(() => [mockWorkflowStep]),
  mockRunWorkflow: vi.fn(() => Promise.resolve({ id: 'exec-001' })),
  mockCancelWorkflow: vi.fn(() => Promise.resolve({ status: 'cancelled' })),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (selector === mockSelectActiveWorkflowId) return mockSelectActiveWorkflowId()
    if (selector === mockSelectIsRunning) return mockSelectIsRunning()
    if (selector === mockSelectRunId) return mockSelectRunId()
    if (selector === mockSelectSteps) return mockSelectSteps()
    return undefined
  }),
  workflowStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectActiveWorkflowId: mockSelectActiveWorkflowId,
    selectSteps: mockSelectSteps,
  },
  workflowExecutionStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectIsRunning: mockSelectIsRunning,
    selectRunId: mockSelectRunId,
    beginRun: mockBeginRun,
  },
  workflowLiveStore: {
    hydrateActive: mockHydrateActive,
  },
}))

vi.mock('@/api', () => ({
  api: {
    workflows: {
      run: mockRunWorkflow,
      cancel: mockCancelWorkflow,
    },
  },
}))

beforeEach(() => {
  mockSelectIsRunning.mockReturnValue(false)
  mockSelectRunId.mockReturnValue(null)
  vi.clearAllMocks()
  vi.useFakeTimers({ shouldAdvanceTime: true })
  mockSelectActiveWorkflowId.mockReturnValue('wf-001')
  mockSelectSteps.mockReturnValue([mockWorkflowStep])
  mockRunWorkflow.mockReturnValue(Promise.resolve({ id: 'exec-001' }))
  mockCancelWorkflow.mockReturnValue(Promise.resolve({ status: 'cancelled' }))
})

describe('RunButton', () => {
  it('returns null when no active workflow', () => {
    mockSelectActiveWorkflowId.mockReturnValue(null)
    const { container } = render(<RunButton />)
    expect(container.innerHTML).toBe('')
  })

  it('renders Run button in idle state', () => {
    render(<RunButton />)
    expect(screen.getByText('Run')).toBeInTheDocument()
  })

  it('calls api.workflows.run on click', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    render(<RunButton />)

    await user.click(screen.getByText('Run'))

    expect(mockRunWorkflow).toHaveBeenCalledOnce()
    expect(mockRunWorkflow).toHaveBeenCalledWith('wf-001', undefined)
  })

  it('calls api.workflows.run with initial_input from context step', async () => {
    const contextStep = { ...mockWorkflowStep, execution_mode: 'context' as const, prompt_template: 'Summarize this' }
    mockSelectSteps.mockReturnValue([contextStep])
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    render(<RunButton />)

    await user.click(screen.getByText('Run'))

    expect(mockRunWorkflow).toHaveBeenCalledWith('wf-001', { initial_input: 'Summarize this' })
  })

  it('prefers input step over context step for initial_input', async () => {
    const contextStep = { ...mockWorkflowStep, id: 'ctx-1', execution_mode: 'context' as const, prompt_template: 'Context text' }
    const inputStep = { ...mockWorkflowStep, id: 'input-1', execution_mode: 'input' as const, prompt_template: 'Input text' }
    mockSelectSteps.mockReturnValue([contextStep, inputStep])
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    render(<RunButton />)

    await user.click(screen.getByText('Run'))

    expect(mockRunWorkflow).toHaveBeenCalledWith('wf-001', { initial_input: 'Input text' })
  })

  it('shows a Cancel affordance whenever the server reports an active run', () => {
    // Running is server state now, not a local flag set on click — which is why
    // it is still correct after a page refresh.
    mockSelectIsRunning.mockReturnValue(true)

    render(<RunButton />)

    expect(screen.getByText('Cancel')).toBeInTheDocument()
  })

  it('calls api.workflows.cancel when clicked while running', async () => {
    mockSelectIsRunning.mockReturnValue(true)
    mockSelectRunId.mockReturnValue('exec-001')
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })

    render(<RunButton />)
    await user.click(screen.getByText('Cancel'))

    expect(mockCancelWorkflow).toHaveBeenCalledWith('exec-001')
    expect(mockRunWorkflow).not.toHaveBeenCalled()
  })

  it('keeps the failure visible instead of silently resetting', async () => {
    mockRunWorkflow.mockReturnValue(Promise.reject(new Error('network error')))

    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    render(<RunButton />)

    await user.click(screen.getByText('Run'))

    expect(screen.getByText('Failed')).toBeInTheDocument()

    act(() => { vi.advanceTimersByTime(3000) })

    // No timer clears it — the user keeps seeing that the run never started.
    expect(screen.getByText('Failed')).toBeInTheDocument()
  })
})
