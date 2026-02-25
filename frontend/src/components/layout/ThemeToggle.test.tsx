import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ThemeToggle } from './ThemeToggle'
import type { ThemeId } from '@/theme'

const mockSetTheme = vi.hoisted(() => vi.fn())
let mockThemeId: ThemeId = 'linen'

vi.mock('@/hooks/useThemeMode', () => ({
  useThemeMode: () => ({ themeId: mockThemeId, setTheme: mockSetTheme, cycleTheme: vi.fn() }),
}))

describe('ThemeToggle', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockThemeId = 'linen'
  })

  it('renders palette icon button', () => {
    render(<ThemeToggle />)
    expect(screen.getByLabelText('Theme')).toBeInTheDocument()
  })

  it('opens menu with all five themes on click', async () => {
    const user = userEvent.setup()
    render(<ThemeToggle />)

    await user.click(screen.getByLabelText('Theme'))

    expect(screen.getByText('Linen')).toBeInTheDocument()
    expect(screen.getByText('Paper')).toBeInTheDocument()
    expect(screen.getByText('Obsidian')).toBeInTheDocument()
    expect(screen.getByText('Midnight')).toBeInTheDocument()
    expect(screen.getByText('Slate')).toBeInTheDocument()
  })

  it('calls setTheme when a theme is selected', async () => {
    const user = userEvent.setup()
    render(<ThemeToggle />)

    await user.click(screen.getByLabelText('Theme'))
    await user.click(screen.getByText('Midnight'))

    expect(mockSetTheme).toHaveBeenCalledWith('midnight')
  })

  it('closes menu after selection', async () => {
    const user = userEvent.setup()
    render(<ThemeToggle />)

    await user.click(screen.getByLabelText('Theme'))
    await user.click(screen.getByText('Slate'))

    expect(screen.queryByText('Linen')).not.toBeInTheDocument()
  })
})
