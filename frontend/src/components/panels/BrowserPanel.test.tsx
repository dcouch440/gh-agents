import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { BrowserPanel } from './BrowserPanel'

type TestItem = { id: string; name: string; desc: string }

const items: TestItem[] = [
  { id: '1', name: 'Alpha', desc: 'First' },
  { id: '2', name: 'Beta', desc: 'Second' },
  { id: '3', name: 'Gamma', desc: 'Third' },
]

const toRow = (item: TestItem) => ({ primary: item.name, secondary: item.desc })
const matchesQuery = (item: TestItem, query: string) => item.name.toLowerCase().includes(query.toLowerCase())
const isHighlighted = (_item: TestItem) => false

const defaultProps = {
  items,
  loading: false,
  searchPlaceholder: 'Search items...',
  emptyIcon: <span>icon</span>,
  emptyLabel: 'items',
  barColor: '#000',
  toRow,
  matchesQuery,
  isHighlighted,
  onItemClick: null as ((id: string) => void) | null,
}

beforeEach(() => {
  vi.clearAllMocks()
})

describe('BrowserPanel', () => {
  it('renders all items', () => {
    render(<BrowserPanel {...defaultProps} />)
    expect(screen.getByText('Alpha')).toBeInTheDocument()
    expect(screen.getByText('Beta')).toBeInTheDocument()
    expect(screen.getByText('Gamma')).toBeInTheDocument()
  })

  it('renders secondary text for items', () => {
    render(<BrowserPanel {...defaultProps} />)
    expect(screen.getByText('First')).toBeInTheDocument()
    expect(screen.getByText('Second')).toBeInTheDocument()
  })

  it('shows loading spinner when loading', () => {
    render(<BrowserPanel {...defaultProps} loading={true} />)
    expect(screen.getByText('Loading items...')).toBeInTheDocument()
  })

  it('shows empty state when no items', () => {
    render(<BrowserPanel {...defaultProps} items={[]} />)
    expect(screen.getByText('No items found')).toBeInTheDocument()
  })

  it('filters items by search query', async () => {
    const user = userEvent.setup()
    render(<BrowserPanel {...defaultProps} />)

    const input = screen.getByPlaceholderText('Search items...')
    await user.type(input, 'Alph')

    await vi.waitFor(() => {
      expect(screen.getByText('Alpha')).toBeInTheDocument()
      expect(screen.queryByText('Beta')).not.toBeInTheDocument()
      expect(screen.queryByText('Gamma')).not.toBeInTheDocument()
    })
  })

  it('shows contextual empty state for search with no results', async () => {
    const user = userEvent.setup()
    render(<BrowserPanel {...defaultProps} />)

    const input = screen.getByPlaceholderText('Search items...')
    await user.type(input, 'zzz')

    await vi.waitFor(() => {
      expect(screen.getByText('No items matching "zzz"')).toBeInTheDocument()
    })
  })

  it('calls onItemClick when item is clicked', async () => {
    const onClick = vi.fn()
    const user = userEvent.setup()
    render(<BrowserPanel {...defaultProps} onItemClick={onClick} />)

    await user.click(screen.getByText('Beta'))
    expect(onClick).toHaveBeenCalledWith('2')
  })

  it('does not call handler when onItemClick is null', async () => {
    const user = userEvent.setup()
    render(<BrowserPanel {...defaultProps} onItemClick={null} />)

    await user.click(screen.getByText('Beta'))
    // No error thrown — clicks are inert
  })

  it('highlights matching items', () => {
    const highlightAlpha = (item: TestItem) => item.id === '1'
    render(<BrowserPanel {...defaultProps} isHighlighted={highlightAlpha} />)
    // BrowserPanel passes highlight prop to AccentBarRow — verify no errors
    expect(screen.getByText('Alpha')).toBeInTheDocument()
  })
})
