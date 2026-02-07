import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { LoadingSpinner } from './LoadingSpinner'

describe('LoadingSpinner', () => {
  it('renders with default md size', () => {
    const { container } = render(<LoadingSpinner />)
    const spinner = container.querySelector('.MuiCircularProgress-root')
    expect(spinner).toBeInTheDocument()
    // Default size is md = 40px
    expect(spinner).toHaveStyle({ width: '40px', height: '40px' })
  })

  it('renders with specified size', () => {
    const { container } = render(<LoadingSpinner size="lg" />)
    const spinner = container.querySelector('.MuiCircularProgress-root')
    expect(spinner).toBeInTheDocument()
    // lg size = 60px
    expect(spinner).toHaveStyle({ width: '60px', height: '60px' })
  })

  it('does not wrap in centering container by default', () => {
    const { container } = render(<LoadingSpinner />)
    // The root should be an inline-flex box (the spinner wrapper), not a centering flex container
    const root = container.firstElementChild
    expect(root).toHaveStyle({ display: 'inline-flex' })
  })

  it('wraps in centering container when centered', () => {
    const { container } = render(<LoadingSpinner centered />)
    // The outermost element should be the centering container
    const root = container.firstElementChild
    expect(root).toHaveStyle({ display: 'flex', justifyContent: 'center', alignItems: 'center' })
    // The spinner should be nested inside
    const spinner = container.querySelector('.MuiCircularProgress-root')
    expect(spinner).toBeInTheDocument()
  })
})
