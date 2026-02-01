import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Card } from './Card'

describe('Card', () => {
  it('renders children', () => {
    render(<Card><p>Content</p></Card>)
    expect(screen.getByText('Content')).toBeInTheDocument()
  })

  it('does not render title when not provided', () => {
    const { container } = render(<Card><p>Content</p></Card>)
    expect(container.querySelector('.card__title')).not.toBeInTheDocument()
  })

  it('renders title when provided', () => {
    render(<Card title="Stats"><p>Content</p></Card>)
    expect(screen.getByText('Stats')).toBeInTheDocument()
    expect(screen.getByText('Stats').closest('.card__title')).toBeInTheDocument()
  })

  it('applies card class', () => {
    const { container } = render(<Card><p>Hi</p></Card>)
    expect(container.querySelector('.card')).toBeInTheDocument()
  })
})
