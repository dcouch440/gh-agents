import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ThemeToggle } from './ThemeToggle'
import type { ThemeId } from '@/theme'

const mockCycleTheme = vi.hoisted(() => vi.fn())
let mockThemeId: ThemeId = 'linen'

vi.mock('@/hooks/useThemeMode', () => ({
  useThemeMode: () => ({ themeId: mockThemeId, cycleTheme: mockCycleTheme, setTheme: vi.fn() }),
}))

describe('ThemeToggle', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockThemeId = 'linen'
  })

  it('renders sun icon for linen theme', () => {
    render(<ThemeToggle />)
    expect(screen.getByTestId('LightModeOutlinedIcon')).toBeInTheDocument()
  })

  it('renders moon icon for midnight theme', () => {
    mockThemeId = 'midnight'
    render(<ThemeToggle />)
    expect(screen.getByTestId('DarkModeOutlinedIcon')).toBeInTheDocument()
  })

  it('renders contrast icon for slate theme', () => {
    mockThemeId = 'slate'
    render(<ThemeToggle />)
    expect(screen.getByTestId('ContrastOutlinedIcon')).toBeInTheDocument()
  })

  it('calls cycleTheme when clicked', async () => {
    const user = userEvent.setup()
    render(<ThemeToggle />)

    await user.click(screen.getByRole('button'))
    expect(mockCycleTheme).toHaveBeenCalledOnce()
  })
})
