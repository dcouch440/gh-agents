import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { boardStore } from '@/stores/boardStore'
import { workflowStore } from '@/stores/workflowStore'
import { agentTraceStore } from '@/stores/agentTraceStore'
import { createNormalizedMap } from '@/stores/lib'
import { DispatchPanel } from './DispatchPanel'

// Mock hooks that make API calls
vi.mock('../hooks/useDispatchPollAll', () => ({
  useDispatchPollAll: vi.fn(),
}))

describe('DispatchPanel', () => {
  beforeEach(() => {
    boardStore.store.setState({
      status: 'idle',
      error: null,
      lastResponse: null,
      isFirstSubmit: true,
      elementStepMap: {},
      elementEdgeMap: {},
    })
    workflowStore.store.setState({
      steps: createNormalizedMap(),
      edges: createNormalizedMap(),
    })
    agentTraceStore.reset()
  })

  it('renders the panel title', () => {
    render(<DispatchPanel onClose={vi.fn()} />)
    expect(screen.getByText('Activity')).toBeInTheDocument()
  })

  it('renders Dispatch and Run tab buttons', () => {
    render(<DispatchPanel onClose={vi.fn()} />)
    expect(screen.getByText('Dispatch')).toBeInTheDocument()
    expect(screen.getByText('Run')).toBeInTheDocument()
  })

  it('shows dispatch content by default', () => {
    render(<DispatchPanel onClose={vi.fn()} />)
    expect(screen.getByText(/no dispatches yet/i)).toBeInTheDocument()
  })

  it('switches to Run tab on click', () => {
    render(<DispatchPanel onClose={vi.fn()} />)

    fireEvent.click(screen.getByText('Run'))

    expect(screen.getByText(/no execution traces yet/i)).toBeInTheDocument()
  })

  it('calls onClose when close button is clicked', () => {
    const onClose = vi.fn()
    render(<DispatchPanel onClose={onClose} />)

    const closeButton = screen.getByRole('button', { name: /close panel/i })
    closeButton.click()
    expect(onClose).toHaveBeenCalledOnce()
  })
})
