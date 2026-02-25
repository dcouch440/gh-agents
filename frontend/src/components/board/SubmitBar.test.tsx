import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { SubmitBar } from './SubmitBar'

const defaultProps = {
  onSubmit: vi.fn(),
  isSubmitting: false,
  status: 'idle' as const,
  error: null,
  onRun: vi.fn(),
  runStatus: 'idle' as const,
  showDebug: false,
  onToggleDebug: vi.fn(),
}

describe('SubmitBar', () => {
  it('renders a submit button', () => {
    render(<SubmitBar {...defaultProps} />)
    expect(screen.getByRole('button', { name: /submit/i })).toBeInTheDocument()
  })

  it('calls onSubmit when clicked', () => {
    const onSubmit = vi.fn()
    render(<SubmitBar {...defaultProps} onSubmit={onSubmit} />)

    fireEvent.click(screen.getByRole('button', { name: /submit/i }))
    expect(onSubmit).toHaveBeenCalledOnce()
  })

  it('disables button while submitting', () => {
    render(<SubmitBar {...defaultProps} isSubmitting={true} status="submitting" />)
    expect(screen.getByRole('button', { name: /submit/i })).toBeDisabled()
  })

  it('shows error message on error status', () => {
    render(<SubmitBar {...defaultProps} status="error" error="Network timeout" />)
    expect(screen.getByText('Network timeout')).toBeInTheDocument()
  })

  it('does not show error message when idle', () => {
    render(<SubmitBar {...defaultProps} />)
    expect(screen.queryByText(/timeout/i)).not.toBeInTheDocument()
  })

  it('renders a run button', () => {
    render(<SubmitBar {...defaultProps} />)
    expect(screen.getByRole('button', { name: /run/i })).toBeInTheDocument()
  })

  it('calls onRun when run button is clicked', () => {
    const onRun = vi.fn()
    render(<SubmitBar {...defaultProps} onRun={onRun} />)

    fireEvent.click(screen.getByRole('button', { name: /run/i }))
    expect(onRun).toHaveBeenCalledOnce()
  })

  it('disables run button while running', () => {
    render(<SubmitBar {...defaultProps} runStatus="running" />)
    expect(screen.getByRole('button', { name: /running/i })).toBeDisabled()
  })

  it('shows success label after run completes', () => {
    render(<SubmitBar {...defaultProps} runStatus="completed" />)
    expect(screen.getByRole('button', { name: /started/i })).toBeInTheDocument()
  })

  it('shows error label when run fails', () => {
    render(<SubmitBar {...defaultProps} runStatus="error" />)
    expect(screen.getByRole('button', { name: /failed/i })).toBeInTheDocument()
  })
})
