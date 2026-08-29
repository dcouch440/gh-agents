import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { InputNodeRunButton } from './InputNodeRunButton'

const {
  mockSelectActiveWorkflowId,
  mockSelectStepById,
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
  mockSelectStepById: vi.fn(() => () => ({ prompt_template: 'Default input text' })),
  mockRunWorkflow: vi.fn(() => Promise.resolve({ id: 'exec-001' })),
  mockCancelWorkflow: vi.fn(() => Promise.resolve({ status: 'cancelled' })),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (selector === mockSelectActiveWorkflowId) return mockSelectActiveWorkflowId()
    if (selector === mockSelectIsRunning) return mockSelectIsRunning()
    if (selector === mockSelectRunId) return mockSelectRunId()
    if (typeof selector === 'function') return (selector as () => unknown)()
    return undefined
  }),
  workflowStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectActiveWorkflowId: mockSelectActiveWorkflowId,
    selectStepById: mockSelectStepById,
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
  mockSelectStepById.mockReturnValue(() => ({ prompt_template: 'Default input text' }))
  mockRunWorkflow.mockReturnValue(Promise.resolve({ id: 'exec-001' }))
  mockCancelWorkflow.mockReturnValue(Promise.resolve({ status: 'cancelled' }))
})

describe('InputNodeRunButton', () => {
  it('returns null when no active workflow', () => {
    mockSelectActiveWorkflowId.mockReturnValue(null)
    const { container } = render(<InputNodeRunButton stepId="step-1" />)
    expect(container.innerHTML).toBe('')
  })

  it('renders play button in idle state', () => {
    render(<InputNodeRunButton stepId="step-1" />)
    expect(screen.getByRole('button')).toBeInTheDocument()
  })

  it('calls api.workflows.run with initial_input from prompt_template', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    render(<InputNodeRunButton stepId="step-1" />)

    await user.click(screen.getByRole('button'))

    expect(mockRunWorkflow).toHaveBeenCalledOnce()
    expect(mockRunWorkflow).toHaveBeenCalledWith('wf-001', { initial_input: 'Default input text' })
  })

  it('sends undefined body when prompt_template is empty', async () => {
    mockSelectStepById.mockReturnValue(() => ({ prompt_template: '   ' }))
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    render(<InputNodeRunButton stepId="step-1" />)

    await user.click(screen.getByRole('button'))

    expect(mockRunWorkflow).toHaveBeenCalledWith('wf-001', undefined)
  })

  it('stays enabled whenever the server reports an active run, so it can be cancelled', () => {
    mockSelectIsRunning.mockReturnValue(true)

    render(<InputNodeRunButton stepId="step-1" />)

    expect(screen.getByRole('button')).not.toBeDisabled()
  })

  it('calls api.workflows.cancel when clicked while running', async () => {
    mockSelectIsRunning.mockReturnValue(true)
    mockSelectRunId.mockReturnValue('exec-001')
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })

    render(<InputNodeRunButton stepId="step-1" />)
    await user.click(screen.getByRole('button'))

    expect(mockCancelWorkflow).toHaveBeenCalledWith('exec-001')
    expect(mockRunWorkflow).not.toHaveBeenCalled()
  })

  it('stays enabled after a failure so the run can be retried', async () => {
    mockRunWorkflow.mockReturnValue(Promise.reject(new Error('network error')))

    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    render(<InputNodeRunButton stepId="step-1" />)

    await user.click(screen.getByRole('button'))

    expect(screen.getByRole('button')).not.toBeDisabled()
  })
})
