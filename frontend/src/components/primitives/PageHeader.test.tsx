import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PageHeader } from './PageHeader'

describe('PageHeader', () => {
  it('renders the title', () => {
    render(<PageHeader title="Agents" />)
    expect(screen.getByText('Agents')).toBeInTheDocument()
  })

  it('does not render actions wrapper when no children', () => {
    render(<PageHeader title="Agents" />)
    // Only the title heading should be rendered, no extra button containers
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
  })

  it('renders children in actions slot', () => {
    render(
      <PageHeader title="Agents">
        <button>Create</button>
      </PageHeader>,
    )
    expect(screen.getByText('Create')).toBeInTheDocument()
    // The button should be rendered and accessible
    expect(screen.getByRole('button', { name: 'Create' })).toBeInTheDocument()
  })
})
