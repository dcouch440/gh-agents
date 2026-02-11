import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/render'
import { ContextNodeHeader } from './ContextNodeHeader'

describe('ContextNodeHeader', () => {
  it('renders the node name', () => {
    render(<ContextNodeHeader name="System Prompt" />)
    expect(screen.getByText('System Prompt')).toBeInTheDocument()
  })

  it('renders the Context badge', () => {
    render(<ContextNodeHeader name="My Context" />)
    expect(screen.getByText('Context')).toBeInTheDocument()
  })

  it('uses default accent color when none provided', () => {
    render(<ContextNodeHeader name="Test" />)
    // Just verify it renders without error
    expect(screen.getByText('Test')).toBeInTheDocument()
    expect(screen.getByText('Context')).toBeInTheDocument()
  })

  it('uses custom accent color when provided', () => {
    render(<ContextNodeHeader name="Test" accentColor="#00ff00" />)
    expect(screen.getByText('Test')).toBeInTheDocument()
    expect(screen.getByText('Context')).toBeInTheDocument()
  })
})
