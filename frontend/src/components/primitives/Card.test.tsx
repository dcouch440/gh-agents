import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Card } from './Card'

describe('Card', () => {
  it('renders children', () => {
    render(<Card><p>Content</p></Card>)
    expect(screen.getByText('Content')).toBeInTheDocument()
  })

  it('does not render title when not provided', () => {
    render(<Card><p>Content</p></Card>)
    expect(screen.queryByRole('heading')).not.toBeInTheDocument()
  })

  it('renders title when provided', () => {
    render(<Card title="Stats"><p>Content</p></Card>)
    const heading = screen.getByRole('heading', { name: 'Stats' })
    expect(heading).toBeInTheDocument()
  })

  it('applies Paper as root element', () => {
    const { container } = render(<Card><p>Hi</p></Card>)
    expect(container.querySelector('.MuiPaper-root')).toBeInTheDocument()
  })
})
