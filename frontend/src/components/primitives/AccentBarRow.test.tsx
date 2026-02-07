import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AccentBarRow } from './AccentBarRow'

describe('AccentBarRow', () => {
  it('renders primary and secondary text', () => {
    render(
      <AccentBarRow barColor="#3b82f6" primary="Frontend Dev" secondary="frontend" />
    )
    expect(screen.getByText('Frontend Dev')).toBeInTheDocument()
    expect(screen.getByText('frontend')).toBeInTheDocument()
  })

  it('renders without secondary text', () => {
    render(<AccentBarRow barColor="#3b82f6" primary="Frontend Dev" />)
    expect(screen.getByText('Frontend Dev')).toBeInTheDocument()
  })

  it('renders actions when provided', () => {
    render(
      <AccentBarRow
        barColor="#3b82f6"
        primary="Test"
        actions={<button type="button">Edit</button>}
      />
    )
    expect(screen.getByRole('button', { name: 'Edit' })).toBeInTheDocument()
  })

  it('calls onClick when clicked', async () => {
    const user = userEvent.setup()
    const onClick = vi.fn()
    render(
      <AccentBarRow barColor="#3b82f6" primary="Clickable" onClick={onClick} />
    )

    await user.click(screen.getByText('Clickable'))
    expect(onClick).toHaveBeenCalledOnce()
  })

  it('renders children instead of primary/secondary when provided', () => {
    render(
      <AccentBarRow barColor="#3b82f6" primary="Ignored">
        <span data-testid="custom-content">Custom Layout</span>
      </AccentBarRow>
    )
    expect(screen.getByTestId('custom-content')).toBeInTheDocument()
    expect(screen.queryByText('Ignored')).not.toBeInTheDocument()
  })
})
