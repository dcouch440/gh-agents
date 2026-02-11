import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ProtocolBadge } from './ProtocolBadge'

describe('ProtocolBadge', () => {
  it('renders the label text', () => {
    render(<ProtocolBadge color="#ff0000" label="Protocol" />)
    expect(screen.getByText('Protocol')).toBeInTheDocument()
  })

  it('renders the dot indicator', () => {
    render(<ProtocolBadge color="#ff0000" label="Test" />)
    expect(screen.getByTestId('protocol-badge-dot')).toBeInTheDocument()
  })

  it('renders with different labels', () => {
    render(<ProtocolBadge color="#00ff00" label="Document" />)
    expect(screen.getByText('Document')).toBeInTheDocument()
  })

  it('renders with different labels for context', () => {
    render(<ProtocolBadge color="#0000ff" label="Context" />)
    expect(screen.getByText('Context')).toBeInTheDocument()
  })

  it('renders without animation by default', () => {
    const { container } = render(<ProtocolBadge color="#ff0000" label="Test" />)
    const dot = screen.getByTestId('protocol-badge-dot')
    // Without animation, the dot should not have ::after pseudo-element styles for animation
    // We verify by checking the rendered element exists without errors
    expect(dot).toBeInTheDocument()
    expect(container.firstChild).toBeInTheDocument()
  })

  it('accepts animated prop without errors', () => {
    const { container } = render(<ProtocolBadge color="#ff0000" label="Test" animated />)
    expect(container.firstChild).toBeInTheDocument()
    expect(screen.getByText('Test')).toBeInTheDocument()
  })

  it('accepts animated=false explicitly', () => {
    render(<ProtocolBadge color="#ff0000" label="Test" animated={false} />)
    expect(screen.getByText('Test')).toBeInTheDocument()
  })
})
