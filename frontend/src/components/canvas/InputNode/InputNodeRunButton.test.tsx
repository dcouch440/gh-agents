import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, act } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { InputNodeRunButton } from './InputNodeRunButton'

const { mockSelectActiveWorkflowId, mockSelectStepById, mockRunWorkflow } = vi.hoisted(() => ({
  mockSelectActiveWorkflowId: vi.fn<() => string | null>(() => 'wf-001'),
  mockSelectStepById: vi.fn(() => () => ({ prompt_template: 'Default input text' })),
  mockRunWorkflow: vi.fn(() => Promise.resolve({ id: 'exec-001' })),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (selector === mockSelectActiveWorkflowId) return mockSelectActiveWorkflowId()
    if (typeof selector === 'function') return (selector as () => unknown)()
    return undefined
  }),
  workflowStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectActiveWorkflowId: mockSelectActiveWorkflowId,
    selectStepById: mockSelectStepById,
  },
}))

vi.mock('@/api', () => ({
  api: {
    workflows: {
      run: mockRunWorkflow,
    },
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
  vi.useFakeTimers({ shouldAdvanceTime: true })
  mockSelectActiveWorkflowId.mockReturnValue('wf-001')
  mockSelectStepById.mockReturnValue(() => ({ prompt_template: 'Default input text' }))
  mockRunWorkflow.mockReturnValue(Promise.resolve({ id: 'exec-001' }))
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

  it('disables button while running', async () => {
    let resolveRun: () => void = () => {}
    mockRunWorkflow.mockReturnValue(new Promise<void>((r) => { resolveRun = r }))

    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    render(<InputNodeRunButton stepId="step-1" />)

    await user.click(screen.getByRole('button'))

    expect(screen.getByRole('button')).toBeDisabled()

    act(() => { resolveRun() })
  })

  it('recovers to idle after error', async () => {
    mockRunWorkflow.mockReturnValue(Promise.reject(new Error('network error')))

    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    render(<InputNodeRunButton stepId="step-1" />)

    await user.click(screen.getByRole('button'))

    // Button should show error state
    expect(screen.getByRole('button')).toBeInTheDocument()

    act(() => { vi.advanceTimersByTime(3000) })

    // Should recover — button re-enabled
    expect(screen.getByRole('button')).not.toBeDisabled()
  })
})
