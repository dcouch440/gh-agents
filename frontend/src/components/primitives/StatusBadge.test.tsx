import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { StatusBadge } from './StatusBadge'

describe('StatusBadge', () => {
  it('renders the label text', () => {
    render(<StatusBadge label="Active" variant="success" />)
    expect(screen.getByText('Active')).toBeInTheDocument()
  })

  it('renders as a MUI Chip', () => {
    const { container } = render(<StatusBadge label="Idle" variant="neutral" />)
    const chip = container.querySelector('.MuiChip-root')
    expect(chip).toBeInTheDocument()
  })

  it.each(['success', 'warning', 'error', 'info', 'neutral'] as const)(
    'renders with variant %s',
    (variant) => {
      render(<StatusBadge label="Test" variant={variant} />)
      expect(screen.getByText('Test')).toBeInTheDocument()
    },
  )
})
