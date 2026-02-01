import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { KeyValue } from './KeyValue'

describe('KeyValue', () => {
  it('renders label and value', () => {
    render(<KeyValue label="Status">Active</KeyValue>)
    expect(screen.getByText('Status')).toBeInTheDocument()
    expect(screen.getByText('Active')).toBeInTheDocument()
  })

  it('applies kv classes', () => {
    const { container } = render(<KeyValue label="Tier">Worker</KeyValue>)
    expect(container.querySelector('.kv')).toBeInTheDocument()
    expect(container.querySelector('.kv__label')).toBeInTheDocument()
    expect(container.querySelector('.kv__value')).toBeInTheDocument()
  })
})
