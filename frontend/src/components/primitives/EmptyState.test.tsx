import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { EmptyState } from './EmptyState'

describe('EmptyState', () => {
  it('renders the message', () => {
    render(<EmptyState message="No items found" />)
    expect(screen.getByText('No items found')).toBeInTheDocument()
  })

  it('does not render icon when not provided', () => {
    const { container } = render(<EmptyState message="Empty" />)
    expect(container.querySelector('.empty-state__icon')).not.toBeInTheDocument()
  })

  it('renders icon when provided', () => {
    render(<EmptyState icon="📭" message="No mail" />)
    expect(screen.getByText('📭')).toBeInTheDocument()
  })
})
