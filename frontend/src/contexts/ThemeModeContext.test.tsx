import { render, screen } from '@testing-library/react'
import { act } from 'react'
import { ThemeModeProvider } from './ThemeModeContext'
import { useThemeMode } from '@/hooks/useThemeMode'

// ── Mocks ────────────────────────────────────────────────────────────────────

vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, LS_THEME: 'test_theme' }
})

const mockStorage = vi.hoisted(() => {
  const store = new Map<string, string>()
  return {
    store,
    getItem: vi.fn((key: string) => store.get(key) ?? null),
    setItem: vi.fn((key: string, value: string) => { store.set(key, value) }),
    removeItem: vi.fn((key: string) => { store.delete(key) }),
    clear: vi.fn(() => { store.clear() }),
    get length() { return store.size },
    key: vi.fn(() => null),
  }
})

const mockMatchMedia = vi.hoisted(() => vi.fn())

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
    mockStorage.store.clear()

    Object.defineProperty(window, 'localStorage', {
      value: mockStorage,
      writable: true,
    })

    mockMatchMedia.mockImplementation((query: string) => ({
      matches: query === '(prefers-color-scheme: dark)',
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }))

    Object.defineProperty(window, 'matchMedia', {
      value: mockMatchMedia,
      writable: true,
    })
  })

  describe('ThemeModeProvider', () => {
    it('defaults to system preference when no stored value', () => {
      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      expect(screen.getByTestId('mode')).toHaveTextContent('dark')
    })

    it('defaults to light when system prefers light', () => {
      mockMatchMedia.mockImplementation((query: string) => ({
        matches: query !== '(prefers-color-scheme: dark)',
        media: query,
        onchange: null,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
      }))

      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      expect(screen.getByTestId('mode')).toHaveTextContent('light')
    })

    it('reads stored mode from localStorage', () => {
      mockStorage.store.set('test_theme', 'light')

      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      expect(screen.getByTestId('mode')).toHaveTextContent('light')
    })

    it('toggles between light and dark mode', () => {
      mockStorage.store.set('test_theme', 'dark')

      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      expect(screen.getByTestId('mode')).toHaveTextContent('dark')

      act(() => {
        screen.getByText('toggle').click()
      })

      expect(screen.getByTestId('mode')).toHaveTextContent('light')
      expect(mockStorage.setItem).toHaveBeenCalledWith('test_theme', 'light')

      act(() => {
        screen.getByText('toggle').click()
      })

      expect(screen.getByTestId('mode')).toHaveTextContent('dark')
      expect(mockStorage.setItem).toHaveBeenCalledWith('test_theme', 'dark')
    })

    it('sets mode directly and persists to localStorage', () => {
      render(
        <ThemeModeProvider>
          <TestConsumer />
        </ThemeModeProvider>,
      )

      act(() => {
        screen.getByText('set-light').click()
      })

      expect(screen.getByTestId('mode')).toHaveTextContent('light')
      expect(mockStorage.setItem).toHaveBeenCalledWith('test_theme', 'light')

      act(() => {
        screen.getByText('set-dark').click()
      })

      expect(screen.getByTestId('mode')).toHaveTextContent('dark')
      expect(mockStorage.setItem).toHaveBeenCalledWith('test_theme', 'dark')
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
