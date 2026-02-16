import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/render'
import { DynamicNodeHeader } from './DynamicNodeHeader'
import { Archetype } from './archetypes'

describe('DynamicNodeHeader', () => {
  it('renders the node name', () => {
    render(<DynamicNodeHeader name="My Workforce" archetype={Archetype.WORKFORCE} subtitle={null} />)
    expect(screen.getByText('My Workforce')).toBeInTheDocument()
  })

  it('renders subtitle when provided', () => {
    render(<DynamicNodeHeader name="Node" archetype={Archetype.WORKFORCE} subtitle="3 agents" />)
    expect(screen.getByText('3 agents')).toBeInTheDocument()
  })

  it('shows fallback text for workforce with no subtitle', () => {
    render(<DynamicNodeHeader name="Node" archetype={Archetype.WORKFORCE} subtitle={null} />)
    expect(screen.getByText('No team')).toBeInTheDocument()
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
    render(<DynamicNodeHeader name="Node" archetype={Archetype.WORKFORCE} subtitle={null} />)
    expect(screen.getByText('Workforce')).toBeInTheDocument()
  })

  it('does not render protocol badge for blank archetype', () => {
    render(<DynamicNodeHeader name="Node" archetype={Archetype.BLANK} subtitle={null} />)
    expect(screen.queryByText('Workforce')).not.toBeInTheDocument()
    expect(screen.queryByText('Room')).not.toBeInTheDocument()
  })

  it('renders expand button when onExpand is provided', () => {
    const onExpand = () => {}
    render(<DynamicNodeHeader name="Node" archetype={Archetype.WORKFORCE} subtitle={null} onExpand={onExpand} />)
    expect(screen.getByRole('button')).toBeInTheDocument()
  })

  it('does not render expand button when onExpand is undefined', () => {
    render(<DynamicNodeHeader name="Node" archetype={Archetype.WORKFORCE} subtitle={null} />)
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
  })
})
