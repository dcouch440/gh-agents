import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/render'
import { DynamicNodeHeader } from './DynamicNodeHeader'
import { Archetype } from './archetypes'

describe('DynamicNodeHeader', () => {
  it('renders the node name', () => {
    render(<DynamicNodeHeader name="My Documenter" archetype={Archetype.DOCUMENTER} subtitle={null} />)
    expect(screen.getByText('My Documenter')).toBeInTheDocument()
  })

  it('renders subtitle when provided', () => {
    render(<DynamicNodeHeader name="Node" archetype={Archetype.DOCUMENTER} subtitle="2 documents" />)
    expect(screen.getByText('2 documents')).toBeInTheDocument()
  })

  it('shows fallback text for documenter with no subtitle', () => {
    render(<DynamicNodeHeader name="Node" archetype={Archetype.DOCUMENTER} subtitle={null} />)
    expect(screen.getByText('No documents')).toBeInTheDocument()
  })

  it('shows fallback text for task_force with no subtitle', () => {
    render(<DynamicNodeHeader name="Node" archetype={Archetype.TASK_FORCE} subtitle={null} />)
    expect(screen.getByText('No agent roster')).toBeInTheDocument()
  })

  it('shows fallback text for room with no subtitle', () => {
    render(<DynamicNodeHeader name="Node" archetype={Archetype.ROOM} subtitle={null} />)
    expect(screen.getByText('No members')).toBeInTheDocument()
  })

  it('shows "Unconfigured" for blank archetype', () => {
    render(<DynamicNodeHeader name="Node" archetype={Archetype.BLANK} subtitle={null} />)
    expect(screen.getByText('Unconfigured')).toBeInTheDocument()
  })

  it('renders protocol badge for non-blank archetypes', () => {
    render(<DynamicNodeHeader name="Node" archetype={Archetype.DOCUMENTER} subtitle={null} />)
    expect(screen.getByText('Documenter')).toBeInTheDocument()
  })

  it('does not render protocol badge for blank archetype', () => {
    render(<DynamicNodeHeader name="Node" archetype={Archetype.BLANK} subtitle={null} />)
    expect(screen.queryByText('Documenter')).not.toBeInTheDocument()
    expect(screen.queryByText('Task Force')).not.toBeInTheDocument()
    expect(screen.queryByText('Room')).not.toBeInTheDocument()
  })

  it('renders expand button when onExpand is provided', () => {
    const onExpand = () => {}
    render(<DynamicNodeHeader name="Node" archetype={Archetype.DOCUMENTER} subtitle={null} onExpand={onExpand} />)
    expect(screen.getByRole('button')).toBeInTheDocument()
  })

  it('does not render expand button when onExpand is undefined', () => {
    render(<DynamicNodeHeader name="Node" archetype={Archetype.DOCUMENTER} subtitle={null} />)
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
  })
})
