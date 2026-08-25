import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi } from 'vitest'
import { MessageErrorNotice } from './MessageErrorNotice'

describe('MessageErrorNotice', () => {
  it('renders the recorded failure', () => {
    render(<MessageErrorNotice error="LLM call failed (round 0): Stream transport error" />)
    expect(screen.getByText(/Stream transport error/)).toBeInTheDocument()
    expect(screen.getByText('NO RESPONSE')).toBeInTheDocument()
  })

  it('resends the message when retry is clicked', async () => {
    const onRetry = vi.fn()
    render(<MessageErrorNotice error="stream died" onRetry={onRetry} />)

    await userEvent.click(screen.getByRole('button', { name: 'Retry' }))
    expect(onRetry).toHaveBeenCalledTimes(1)
  })

  it('omits the retry button when no handler is given', () => {
    render(<MessageErrorNotice error="stream died" />)
    expect(screen.queryByRole('button', { name: 'Retry' })).not.toBeInTheDocument()
  })
})
