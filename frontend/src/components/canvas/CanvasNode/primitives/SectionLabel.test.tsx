import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { SectionLabel } from './SectionLabel'

describe('SectionLabel', () => {
  it('renders the label text', () => {
    render(<SectionLabel label="Inputs" />)
    expect(screen.getByText('Inputs')).toBeInTheDocument()
  })

  it('renders different labels', () => {
    const { rerender } = render(<SectionLabel label="Tools" />)
    expect(screen.getByText('Tools')).toBeInTheDocument()

    rerender(<SectionLabel label="Ports" />)
    expect(screen.getByText('Ports')).toBeInTheDocument()
  })

  it('renders empty string without error', () => {
    const { container } = render(<SectionLabel label="" />)
    expect(container.firstChild).toBeInTheDocument()
  })
})
