// ============================================================================
// uiStore — Centralized UI state (theme, sidebar, toasts, command palette)
// ============================================================================

import { createStore, lsGet, lsSet } from './lib'
import { LS_THEME } from '@/constants'
import { isValidThemeId, DEFAULT_THEME_ID, THEME_IDS } from '@/theme'
import type { ThemeId } from '@/theme'

// ── Types ────────────────────────────────────────────────────────────────────

type ToastType = 'success' | 'error' | 'warning' | 'info'

type Toast = {
  id: string
  message: string
  type: ToastType
  duration: number | null
  createdAt: number
}

type AddToastOptions = {
  message: string
  type?: ToastType
  duration?: number | null
}

type UIState = {
  theme: ThemeId
  toasts: Toast[]
  commandPaletteOpen: boolean
}

// ── Initialization helpers ───────────────────────────────────────────────────

const getSystemPreference = (): ThemeId => {
  try {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'midnight' : 'linen'
  } catch {
    return DEFAULT_THEME_ID
  }
}

const getInitialTheme = (): ThemeId => {
  const stored = lsGet(LS_THEME)
  if (stored !== null && isValidThemeId(stored)) return stored
  // Migrate legacy values
  if (stored === 'light') return 'linen'
  if (stored === 'dark') return 'midnight'
  return getSystemPreference()
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<UIState>(() => ({
  theme: getInitialTheme(),
  toasts: [],
  commandPaletteOpen: false,
}))

// ── ID generator ─────────────────────────────────────────────────────────────

let toastCounter = 0
const nextToastId = (): string => `toast-${++toastCounter}`

// ── Selectors ────────────────────────────────────────────────────────────────

const selectTheme = (s: UIState): ThemeId => s.theme

const selectToasts = (s: UIState): Toast[] => s.toasts

const selectCommandPaletteOpen = (s: UIState): boolean => s.commandPaletteOpen

// ── Theme ────────────────────────────────────────────────────────────────────

const setTheme = (id: ThemeId): void => {
  store.setState({ theme: id })
  lsSet(LS_THEME, id)
  document.documentElement.setAttribute('data-theme', id)
}

const cycleTheme = (): void => {
  const current = store.getState().theme
  const idx = THEME_IDS.indexOf(current)
  const next = THEME_IDS[(idx + 1) % THEME_IDS.length]!
  setTheme(next)
}

// ── Toasts ───────────────────────────────────────────────────────────────────

const DEFAULT_TOAST_DURATION = 5000

const addToast = (opts: AddToastOptions): string => {
  const id = nextToastId()
  const toast: Toast = {
    id,
    message: opts.message,
    type: opts.type ?? 'info',
    duration: opts.duration === undefined ? DEFAULT_TOAST_DURATION : opts.duration,
    createdAt: Date.now(),
  }
  store.setState((s) => ({ toasts: [...s.toasts, toast] }))
  return id
}

const dismissToast = (id: string): void => {
  store.setState((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }))
}

// ── Command Palette ──────────────────────────────────────────────────────────

const openCommandPalette = (): void => {
  store.setState({ commandPaletteOpen: true })
}

const closeCommandPalette = (): void => {
  store.setState({ commandPaletteOpen: false })
}

const toggleCommandPalette = (): void => {
  store.setState((s) => ({ commandPaletteOpen: !s.commandPaletteOpen }))
}

// ── System theme listener ────────────────────────────────────────────────────

let systemThemeCleanup: (() => void) | null = null

const initSystemThemeListener = (): (() => void) => {
  if (systemThemeCleanup) return systemThemeCleanup

  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  const handler = (e: MediaQueryListEvent) => {
    const stored = lsGet(LS_THEME)
    if (!stored) {
      store.setState({ theme: e.matches ? 'midnight' : 'linen' })
    }
  }
  mediaQuery.addEventListener('change', handler)

  systemThemeCleanup = () => {
    mediaQuery.removeEventListener('change', handler)
    systemThemeCleanup = null
  }

  return systemThemeCleanup
}

// ── Export ────────────────────────────────────────────────────────────────────

export const uiStore = {
  store,
  selectTheme,
  selectToasts,
  selectCommandPaletteOpen,
  setTheme,
  cycleTheme,
  addToast,
  dismissToast,
  openCommandPalette,
  closeCommandPalette,
  toggleCommandPalette,
  initSystemThemeListener,
}

export type { UIState, ThemeId, Toast, ToastType, AddToastOptions }
