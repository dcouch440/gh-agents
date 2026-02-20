import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/render'
import { BadgeList } from './BadgeList'

describe('BadgeList', () => {
  it('renders all items', () => {
    render(<BadgeList items={['Alpha', 'Beta', 'Gamma']} />)
    expect(screen.getByText('Alpha')).toBeInTheDocument()
    expect(screen.getByText('Beta')).toBeInTheDocument()
    expect(screen.getByText('Gamma')).toBeInTheDocument()
  })

  it('renders nothing when items is empty', () => {
    const { container } = render(<BadgeList items={[]} />)
    // The wrapper box still exists but has no children
    const wrapper = container.firstChild as HTMLElement
    expect(wrapper.childElementCount).toBe(0)
  })

  it('renders a single item', () => {
    render(<BadgeList items={['Only']} />)
    expect(screen.getByText('Only')).toBeInTheDocument()
  })

  it('accepts custom badgeSx', () => {
    const { container } = render(
      <BadgeList items={['Test']} badgeSx={{ color: 'red' }} />,
    )
    expect(container.firstChild).toBeInTheDocument()
    expect(screen.getByText('Test')).toBeInTheDocument()
  })

  it('handles duplicate item names', () => {
    render(<BadgeList items={['Dup', 'Dup', 'Dup']} />)
    expect(screen.getAllByText('Dup')).toHaveLength(3)
  })
})
