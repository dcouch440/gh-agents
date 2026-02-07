import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { KeyValue } from './KeyValue'

describe('KeyValue', () => {
  it('renders label and value', () => {
    render(<KeyValue label="Status">Active</KeyValue>)
    expect(screen.getByText('Status')).toBeInTheDocument()
    expect(screen.getByText('Active')).toBeInTheDocument()
  })

  it('renders label as caption and value as body2', () => {
    render(<KeyValue label="Tier">Worker</KeyValue>)
    const label = screen.getByText('Tier')
    const value = screen.getByText('Worker')
    expect(label).toBeInTheDocument()
    expect(value).toBeInTheDocument()
    // Label uses caption variant (rendered as div)
    expect(label.tagName).toBe('DIV')
    // Value uses body2 variant (rendered as div)
    expect(value.tagName).toBe('DIV')
  })
})
