import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { LoadingSpinner } from './LoadingSpinner'

describe('LoadingSpinner', () => {
  it('renders with default md size', () => {
    const { container } = render(<LoadingSpinner />)
    expect(container.querySelector('.spinner--md')).toBeInTheDocument()
  })

  it('renders with specified size', () => {
    const { container } = render(<LoadingSpinner size="lg" />)
    expect(container.querySelector('.spinner--lg')).toBeInTheDocument()
  })

  it('does not wrap in container by default', () => {
    const { container } = render(<LoadingSpinner />)
    expect(container.querySelector('.spinner-container')).not.toBeInTheDocument()
  })

  it('wraps in spinner-container when centered', () => {
    const { container } = render(<LoadingSpinner centered />)
    expect(container.querySelector('.spinner-container')).toBeInTheDocument()
    expect(container.querySelector('.spinner-container .spinner--md')).toBeInTheDocument()
  })
})
