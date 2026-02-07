import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PropertyRow } from './PropertyRow'

describe('PropertyRow', () => {
  it('renders label and value', () => {
    render(<PropertyRow label="Model" value="claude-sonnet-4" />)
    expect(screen.getByText('Model')).toBeInTheDocument()
    expect(screen.getByText('claude-sonnet-4')).toBeInTheDocument()
  })

  it('renders children instead of value when provided', () => {
    render(
      <PropertyRow label="Toggle">
        <span data-testid="custom">Custom</span>
      </PropertyRow>
    )
    expect(screen.getByText('Toggle')).toBeInTheDocument()
    expect(screen.getByTestId('custom')).toBeInTheDocument()
  })

  it('renders value as null gracefully', () => {
    render(<PropertyRow label="Empty" />)
    expect(screen.getByText('Empty')).toBeInTheDocument()
  })

  it('applies monospace font when mono is true', () => {
    render(<PropertyRow label="Type" value="llm-call" mono />)
    const value = screen.getByText('llm-call')
    expect(value).toHaveStyle({ fontFamily: 'monospace' })
  })
})
