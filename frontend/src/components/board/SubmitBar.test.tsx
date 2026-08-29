import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { SubmitBar } from './SubmitBar'

const defaultProps = {
  onGenerate: vi.fn(),
  onCancelGenerate: vi.fn(),
  isGenerating: false,
  onRun: vi.fn(),
  onCancelRun: vi.fn(),
  runStatus: 'idle' as const,
  showDebug: false,
  onToggleDebug: vi.fn(),
}

describe('SubmitBar', () => {
  it('renders a design button', () => {
    render(<SubmitBar {...defaultProps} />)
    expect(screen.getByRole('button', { name: /^design$/i })).toBeInTheDocument()
  })

  it('calls onGenerate when clicked', () => {
    const onGenerate = vi.fn()
    render(<SubmitBar {...defaultProps} onGenerate={onGenerate} />)

    fireEvent.click(screen.getByRole('button', { name: /^design$/i }))
    expect(onGenerate).toHaveBeenCalledOnce()
  })

  it('calls onCancelGenerate instead of onGenerate while generating', () => {
    const onGenerate = vi.fn()
    const onCancelGenerate = vi.fn()
    render(<SubmitBar {...defaultProps} onGenerate={onGenerate} onCancelGenerate={onCancelGenerate} isGenerating />)

    fireEvent.click(screen.getByRole('button', { name: /^cancel$/i }))
    expect(onCancelGenerate).toHaveBeenCalledOnce()
    expect(onGenerate).not.toHaveBeenCalled()
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

  it('keeps the run button enabled and routes clicks to cancel while running', () => {
    const onRun = vi.fn()
    const onCancelRun = vi.fn()
    render(<SubmitBar {...defaultProps} onRun={onRun} onCancelRun={onCancelRun} runStatus="running" />)

    const button = screen.getByRole('button', { name: /cancel/i })
    expect(button).not.toBeDisabled()
    fireEvent.click(button)
    expect(onCancelRun).toHaveBeenCalledOnce()
    expect(onRun).not.toHaveBeenCalled()
  })

  it('returns to the idle label once a run is no longer active', () => {
    // There is no transient "Started!" state any more — the button tracks the
    // server's isRunning flag, so it reads Run again the moment a run ends.
    render(<SubmitBar {...defaultProps} runStatus="idle" />)
    expect(screen.getByRole('button', { name: /^run$/i })).toBeInTheDocument()
  })

  it('shows error label when run fails', () => {
    render(<SubmitBar {...defaultProps} runStatus="error" />)
    expect(screen.getByRole('button', { name: /failed/i })).toBeInTheDocument()
  })
})
