import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ApproveButton } from './ApproveButton'

describe('ApproveButton', () => {
  it('renders "Approve" text in default state', () => {
    render(<ApproveButton onApprove={vi.fn()} loading={false} disabled={false} />)
    expect(screen.getByText('Approve')).toBeInTheDocument()
  })

  it('renders "Approving..." text when loading', () => {
    render(<ApproveButton onApprove={vi.fn()} loading={true} disabled={false} />)
    expect(screen.getByText('Approving...')).toBeInTheDocument()
  })

  it('is disabled when loading', () => {
    render(<ApproveButton onApprove={vi.fn()} loading={true} disabled={false} />)
    expect(screen.getByRole('button')).toBeDisabled()
  })

  it('is disabled when disabled prop is true', () => {
    render(<ApproveButton onApprove={vi.fn()} loading={false} disabled={true} />)
    expect(screen.getByRole('button')).toBeDisabled()
  })

  it('calls onApprove when clicked', async () => {
    const user = userEvent.setup()
    const onApprove = vi.fn()
    render(<ApproveButton onApprove={onApprove} loading={false} disabled={false} />)

    await user.click(screen.getByRole('button'))
    expect(onApprove).toHaveBeenCalledOnce()
  })
})
