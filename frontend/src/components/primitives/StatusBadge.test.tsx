import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { StatusBadge } from './StatusBadge'

describe('StatusBadge', () => {
  it('renders the label text', () => {
    render(<StatusBadge label="Active" variant="success" />)
    expect(screen.getByText('Active')).toBeInTheDocument()
  })

  it('applies the correct variant class', () => {
    const { container } = render(<StatusBadge label="Idle" variant="neutral" />)
    const badge = container.querySelector('.badge')
    expect(badge).toHaveClass('badge--neutral')
  })

  it.each(['success', 'warning', 'error', 'info', 'neutral'] as const)(
    'renders with variant %s',
    (variant) => {
      const { container } = render(<StatusBadge label="Test" variant={variant} />)
      expect(container.querySelector(`.badge--${variant}`)).toBeInTheDocument()
    },
  )
})
