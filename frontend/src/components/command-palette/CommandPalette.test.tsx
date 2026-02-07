import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { CommandPalette } from './CommandPalette'

const mockSetQuery = vi.hoisted(() => vi.fn())
const mockHandleKeyDown = vi.hoisted(() => vi.fn())
const mockClosePalette = vi.hoisted(() => vi.fn())

let mockOpen = true
let mockQuery = ''
let mockFilteredCommands: Array<{
  id: string
  label: string
  description: string
  group: string
  action: () => void
  shortcut?: string
}> = []

vi.mock('@/hooks/useCommandPalette', () => ({
  useCommandPalette: () => ({
    open: mockOpen,
    query: mockQuery,
    setQuery: mockSetQuery,
    selectedIndex: 0,
    filteredCommands: mockFilteredCommands,
    handleKeyDown: mockHandleKeyDown,
    closePalette: mockClosePalette,
  }),
}))

vi.mock('@/contexts/CommandPaletteContext', () => ({
  CommandPaletteContext: {
    Provider: ({ children }: { children: React.ReactNode }) => children,
    Consumer: () => null,
    _currentValue: { addRecent: vi.fn() },
  },
}))

describe('CommandPalette', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockOpen = true
    mockQuery = ''
    mockFilteredCommands = []
  })

  it('does not render dialog content when closed', () => {
    mockOpen = false
    render(<CommandPalette />)
    expect(screen.queryByPlaceholderText('Type a command or search...')).not.toBeInTheDocument()
  })

  it('renders search input when open', () => {
    render(<CommandPalette />)
    expect(screen.getByPlaceholderText('Type a command or search...')).toBeInTheDocument()
  })

  it('renders ESC hint', () => {
    render(<CommandPalette />)
    expect(screen.getByText('ESC')).toBeInTheDocument()
  })

  it('shows "No results found" when commands are empty', () => {
    render(<CommandPalette />)
    expect(screen.getByText('No results found')).toBeInTheDocument()
  })

  it('renders grouped commands when commands exist', () => {
    mockFilteredCommands = [
      {
        id: 'cmd-1',
        label: 'Go to Dashboard',
        description: 'Navigate to dashboard',
        group: 'navigation',
        action: vi.fn(),
      },
      {
        id: 'cmd-2',
        label: 'Toggle Theme',
        description: 'Switch dark/light mode',
        group: 'actions',
        action: vi.fn(),
      },
    ]
    render(<CommandPalette />)
    expect(screen.getByText('Go to Dashboard')).toBeInTheDocument()
    expect(screen.getByText('Toggle Theme')).toBeInTheDocument()
  })

  it('renders listbox role for results', () => {
    render(<CommandPalette />)
    expect(screen.getByRole('listbox')).toBeInTheDocument()
  })
})
