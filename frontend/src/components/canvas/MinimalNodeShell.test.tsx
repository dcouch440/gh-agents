import { describe, it, expect } from 'vitest'
import { screen } from '@testing-library/react'
import { render } from '@/test/render'
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

  it('renders the flat node shell without accent stripe', () => {
    const { container } = render(<MinimalNodeShell {...baseProps} />)
    const boxes = container.querySelectorAll('div')
    // Outer box > label container box (no stripe)
    expect(boxes.length).toBeGreaterThanOrEqual(2)
  })

  it('displays the provided label', () => {
    render(<MinimalNodeShell {...baseProps} label="My Custom Name" />)
    expect(screen.getByText('My Custom Name')).toBeInTheDocument()
  })
})
