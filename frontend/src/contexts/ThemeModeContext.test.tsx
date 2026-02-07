import { render, screen } from '@testing-library/react'
import { act } from 'react'
import { ThemeModeProvider } from './ThemeModeContext'
import { useThemeMode } from '@/hooks/useThemeMode'
import { uiStore } from '@/stores/uiStore'

// ── Test consumer ────────────────────────────────────────────────────────────

function TestConsumer() {
  const { mode, toggleMode, setMode } = useThemeMode()

  return (
    <div>
      <div data-testid="mode">{mode}</div>
      <button onClick={toggleMode}>toggle</button>
      <button onClick={() => setMode('light')}>set-light</button>
      <button onClick={() => setMode('dark')}>set-dark</button>
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('ThemeModeContext', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    uiStore.store.setState({
      theme: 'light',
      sidebarCollapsed: false,
      toasts: [],
      commandPaletteOpen: false,
    })
  })

  describe('ThemeModeProvider', () => {
    it('renders with current store theme', () => {
      uiStore.store.setState({ theme: 'dark' })

      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      expect(screen.getByTestId('mode')).toHaveTextContent('dark')
    })

    it('defaults to light theme', () => {
      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      expect(screen.getByTestId('mode')).toHaveTextContent('light')
    })

    it('toggles between light and dark mode', () => {
      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      expect(screen.getByTestId('mode')).toHaveTextContent('light')

      act(() => {
        screen.getByText('toggle').click()
      })

      expect(screen.getByTestId('mode')).toHaveTextContent('dark')

      act(() => {
        screen.getByText('toggle').click()
      })

      expect(screen.getByTestId('mode')).toHaveTextContent('light')
    })

    it('sets mode directly via setMode', () => {
      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      act(() => {
        screen.getByText('set-dark').click()
      })

      expect(screen.getByTestId('mode')).toHaveTextContent('dark')

      act(() => {
        screen.getByText('set-light').click()
      })

      expect(screen.getByTestId('mode')).toHaveTextContent('light')
    })

    it('throws when useThemeMode is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow(
        'useThemeMode must be used within ThemeModeProvider',
      )
      spy.mockRestore()
    })
  })
})
