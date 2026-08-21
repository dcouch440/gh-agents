import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { SubmitBar } from './SubmitBar'

const defaultProps = {
  onGenerate: vi.fn(),
  isGenerating: false,
  onRun: vi.fn(),
  runStatus: 'idle' as const,
  showDebug: false,
  onToggleDebug: vi.fn(),
}

describe('SubmitBar', () => {
  it('renders a generate button', () => {
    render(<SubmitBar {...defaultProps} />)
    expect(screen.getByRole('button', { name: /generate/i })).toBeInTheDocument()
  })

  it('calls onGenerate when clicked', () => {
    const onGenerate = vi.fn()
    render(<SubmitBar {...defaultProps} onGenerate={onGenerate} />)

    fireEvent.click(screen.getByRole('button', { name: /generate/i }))
    expect(onGenerate).toHaveBeenCalledOnce()
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
