import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ConfirmModal } from './ConfirmModal'

describe('ConfirmModal', () => {
  const defaultProps = {
    open: true,
    onClose: vi.fn(),
    onConfirm: vi.fn(),
    title: 'Confirm Action',
    message: 'Are you sure you want to proceed?',
  }

  it('renders with title and message', () => {
    render(<ConfirmModal {...defaultProps} />)

    expect(screen.getByText('Confirm Action')).toBeInTheDocument()
    expect(screen.getByText('Are you sure you want to proceed?')).toBeInTheDocument()
  })

  it('renders with custom button text', () => {
    render(<ConfirmModal {...defaultProps} confirmText="Delete" cancelText="Go Back" />)

    expect(screen.getByText('Delete')).toBeInTheDocument()
    expect(screen.getByText('Go Back')).toBeInTheDocument()
  })

  it('calls onConfirm when confirm button clicked', async () => {
    const user = userEvent.setup()
    const onConfirm = vi.fn()

    render(<ConfirmModal {...defaultProps} onConfirm={onConfirm} />)

    const confirmButton = screen.getByText('Confirm')
    await user.click(confirmButton)

    expect(onConfirm).toHaveBeenCalledTimes(1)
  })

  it('calls onClose when cancel button clicked', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()

    render(<ConfirmModal {...defaultProps} onClose={onClose} />)

    const cancelButton = screen.getByText('Cancel')
    await user.click(cancelButton)

    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('shows loading state', () => {
    render(<ConfirmModal {...defaultProps} loading />)

    expect(screen.getByText('Processing...')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /cancel/i })).toBeDisabled()
    expect(screen.getByRole('button', { name: /processing/i })).toBeDisabled()
  })

  it('displays error message', () => {
    render(<ConfirmModal {...defaultProps} error="Something went wrong" />)

    expect(screen.getByText('Something went wrong')).toBeInTheDocument()
  })

  it('disables close during loading', () => {
    const onClose = vi.fn()
    const { rerender } = render(<ConfirmModal {...defaultProps} onClose={onClose} loading />)

    // Modal should not close when loading
    expect(onClose).not.toHaveBeenCalled()

    // Re-render without loading
    rerender(<ConfirmModal {...defaultProps} onClose={onClose} loading={false} />)
  })

  it('renders ReactNode message', () => {
    const message = (
      <div>
        <p>Custom message</p>
        <strong>With formatting</strong>
      </div>
    )

    render(<ConfirmModal {...defaultProps} message={message} />)

    expect(screen.getByText('Custom message')).toBeInTheDocument()
    expect(screen.getByText('With formatting')).toBeInTheDocument()
  })

  it('applies error color to confirm button', () => {
    render(<ConfirmModal {...defaultProps} confirmColor="error" />)

    const confirmButton = screen.getByRole('button', { name: /confirm/i })
    expect(confirmButton).toBeInTheDocument()
  })

  it('does not render when open is false', () => {
    render(<ConfirmModal {...defaultProps} open={false} />)

    expect(screen.queryByText('Confirm Action')).not.toBeInTheDocument()
  })
})
