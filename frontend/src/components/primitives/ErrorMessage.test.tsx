import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { ErrorMessage } from './ErrorMessage'

describe('ErrorMessage', () => {
  it('renders the error message', () => {
    render(<ErrorMessage message="Something went wrong" />)
    expect(screen.getByText('Something went wrong')).toBeInTheDocument()
  })

  it('does not render retry button when onRetry is not provided', () => {
    render(<ErrorMessage message="Oops" />)
    expect(screen.queryByText('Retry')).not.toBeInTheDocument()
  })

  it('does not render retry button when onRetry is null', () => {
    render(<ErrorMessage message="Oops" onRetry={null} />)
    expect(screen.queryByText('Retry')).not.toBeInTheDocument()
  })

  it('renders retry button when onRetry is provided', () => {
    render(<ErrorMessage message="Oops" onRetry={() => undefined} />)
    expect(screen.getByText('Retry')).toBeInTheDocument()
  })

  it('calls onRetry when retry button is clicked', () => {
    const onRetry = vi.fn()
    render(<ErrorMessage message="Oops" onRetry={onRetry} />)
    fireEvent.click(screen.getByText('Retry'))
    expect(onRetry).toHaveBeenCalledOnce()
  })
})
