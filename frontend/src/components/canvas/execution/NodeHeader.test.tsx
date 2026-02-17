import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'

import { NodeHeader } from './NodeHeader'

const defaultProps = {
  accentColor: '#06b6d4',
  icon: <span data-testid="icon">I</span>,
}

describe('NodeHeader', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders title and subtitle', () => {
    render(
      <NodeHeader {...defaultProps} title="Test" subtitle="Sub" />,
    )
    expect(screen.getByText('Test')).toBeInTheDocument()
    expect(screen.getByText('Sub')).toBeInTheDocument()
  })

  it('does not render subtitle when null', () => {
    render(
      <NodeHeader {...defaultProps} title="Test" subtitle={null} />,
    )
    expect(screen.getByText('Test')).toBeInTheDocument()
    expect(screen.queryByText('Sub')).not.toBeInTheDocument()
  })

  it('renders badge when provided', () => {
    render(
      <NodeHeader
        {...defaultProps}
        title="Test"
        subtitle={null}
        badge={<span data-testid="badge">B</span>}
      />,
    )
    expect(screen.getByTestId('badge')).toBeInTheDocument()
  })

  it('renders actions when provided', () => {
    render(
      <NodeHeader
        {...defaultProps}
        title="Test"
        subtitle={null}
        actions={<button data-testid="action">X</button>}
      />,
    )
    expect(screen.getByTestId('action')).toBeInTheDocument()
  })

  it('renders icon', () => {
    render(
      <NodeHeader {...defaultProps} title="Test" subtitle={null} />,
    )
    expect(screen.getByTestId('icon')).toBeInTheDocument()
  })
})
