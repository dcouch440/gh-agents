import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { SubmitBar } from './SubmitBar'

describe('SubmitBar', () => {
  it('renders a submit button', () => {
    render(<SubmitBar onSubmit={vi.fn()} isSubmitting={false} status="idle" error={null} />)
    expect(screen.getByRole('button', { name: /submit/i })).toBeInTheDocument()
  })

  it('calls onSubmit when clicked', () => {
    const onSubmit = vi.fn()
    render(<SubmitBar onSubmit={onSubmit} isSubmitting={false} status="idle" error={null} />)

    fireEvent.click(screen.getByRole('button', { name: /submit/i }))
    expect(onSubmit).toHaveBeenCalledOnce()
  })

  it('disables button while submitting', () => {
    render(<SubmitBar onSubmit={vi.fn()} isSubmitting={true} status="submitting" error={null} />)
    expect(screen.getByRole('button', { name: /submit/i })).toBeDisabled()
  })

  it('shows error message on error status', () => {
    render(<SubmitBar onSubmit={vi.fn()} isSubmitting={false} status="error" error="Network timeout" />)
    expect(screen.getByText('Network timeout')).toBeInTheDocument()
  })

  it('does not show error message when idle', () => {
    render(<SubmitBar onSubmit={vi.fn()} isSubmitting={false} status="idle" error={null} />)
    expect(screen.queryByText(/timeout/i)).not.toBeInTheDocument()
  })
})
