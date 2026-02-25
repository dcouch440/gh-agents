import { render, screen } from '@testing-library/react'
import { act } from 'react'
import { ThemeModeProvider } from './ThemeModeContext'
import { useThemeMode } from '@/hooks/useThemeMode'
import { uiStore } from '@/stores/uiStore'

// ── Test consumer ────────────────────────────────────────────────────────────

function TestConsumer() {
  const { themeId, cycleTheme, setTheme } = useThemeMode()

  return (
    <div>
      <div data-testid="theme">{themeId}</div>
      <button onClick={cycleTheme}>cycle</button>
      <button onClick={() => setTheme('linen')}>set-linen</button>
      <button onClick={() => setTheme('midnight')}>set-midnight</button>
      <button onClick={() => setTheme('slate')}>set-slate</button>
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('ThemeModeContext', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    uiStore.store.setState({
      theme: 'linen',
      toasts: [],
      commandPaletteOpen: false,
    })
  })

  describe('ThemeModeProvider', () => {
    it('renders with current store theme', () => {
      uiStore.store.setState({ theme: 'midnight' })

      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      expect(screen.getByTestId('theme')).toHaveTextContent('midnight')
    })

    it('defaults to linen theme', () => {
      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      expect(screen.getByTestId('theme')).toHaveTextContent('linen')
    })

    it('cycles through themes: linen → paper → obsidian → midnight → slate → linen', () => {
      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      expect(screen.getByTestId('theme')).toHaveTextContent('linen')

      act(() => {
        screen.getByText('cycle').click()
      })

      expect(screen.getByTestId('theme')).toHaveTextContent('paper')

      act(() => {
        screen.getByText('cycle').click()
      })

      expect(screen.getByTestId('theme')).toHaveTextContent('obsidian')

      act(() => {
        screen.getByText('cycle').click()
      })

      expect(screen.getByTestId('theme')).toHaveTextContent('midnight')

      act(() => {
        screen.getByText('cycle').click()
      })

      expect(screen.getByTestId('theme')).toHaveTextContent('slate')

      act(() => {
        screen.getByText('cycle').click()
      })

      expect(screen.getByTestId('theme')).toHaveTextContent('linen')
    })

    it('sets theme directly via setTheme', () => {
      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      act(() => {
        screen.getByText('set-midnight').click()
      })

      expect(screen.getByTestId('theme')).toHaveTextContent('midnight')

      act(() => {
        screen.getByText('set-slate').click()
      })

      expect(screen.getByTestId('theme')).toHaveTextContent('slate')

      act(() => {
        screen.getByText('set-linen').click()
      })

      expect(screen.getByTestId('theme')).toHaveTextContent('linen')
    })

    it('throws when useThemeMode is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useThemeMode must be used within ThemeModeProvider')
      spy.mockRestore()
    })
  })
})
