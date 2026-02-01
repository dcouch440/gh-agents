import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PageHeader } from './PageHeader'

describe('PageHeader', () => {
  it('renders the title', () => {
    render(<PageHeader title="Agents" />)
    expect(screen.getByText('Agents')).toBeInTheDocument()
  })

  it('does not render actions wrapper when no children', () => {
    const { container } = render(<PageHeader title="Agents" />)
    expect(container.querySelector('.page-header__actions')).not.toBeInTheDocument()
  })

  it('renders children in actions slot', () => {
    render(
      <PageHeader title="Agents">
        <button>Create</button>
      </PageHeader>,
    )
    expect(screen.getByText('Create')).toBeInTheDocument()
    expect(screen.getByText('Create').closest('.page-header__actions')).toBeInTheDocument()
  })
})
