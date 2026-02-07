import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ThemeToggle } from './ThemeToggle'

const mockToggleMode = vi.hoisted(() => vi.fn())
let mockMode: 'light' | 'dark' = 'light'

vi.mock('@/hooks/useThemeMode', () => ({
  useThemeMode: () => ({ mode: mockMode, toggleMode: mockToggleMode }),
}))

describe('ThemeToggle', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockMode = 'light'
  })

  it('renders dark mode icon when in light mode', () => {
    render(<ThemeToggle />)
    expect(screen.getByTestId('DarkModeOutlinedIcon')).toBeInTheDocument()
  })

  it('renders light mode icon when in dark mode', () => {
    mockMode = 'dark'
    render(<ThemeToggle />)
    expect(screen.getByTestId('LightModeOutlinedIcon')).toBeInTheDocument()
  })

  it('calls toggleMode when clicked', async () => {
    const user = userEvent.setup()
    render(<ThemeToggle />)

    await user.click(screen.getByRole('button'))
    expect(mockToggleMode).toHaveBeenCalledOnce()
  })
})
