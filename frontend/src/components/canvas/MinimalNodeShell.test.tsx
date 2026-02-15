import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MinimalNodeShell } from './MinimalNodeShell'

const baseProps = {
  label: 'Test Node',
  accentColor: '#3b82f6',
  borderColor: '#555',
  boxShadow: 'none',
}

describe('MinimalNodeShell', () => {
  it('renders the label text', () => {
    render(<MinimalNodeShell {...baseProps} />)
    expect(screen.getByText('Test Node')).toBeInTheDocument()
  })

  it('renders the accent stripe', () => {
    const { container } = render(<MinimalNodeShell {...baseProps} />)
    const boxes = container.querySelectorAll('div')
    // Outer box > stripe box + label container box
    expect(boxes.length).toBeGreaterThanOrEqual(3)
  })

  it('displays the provided label', () => {
    render(<MinimalNodeShell {...baseProps} label="My Custom Name" />)
    expect(screen.getByText('My Custom Name')).toBeInTheDocument()
  })
})
