// ============================================================================
// uiStore — Centralized UI state (theme, sidebar, toasts, command palette)
// ============================================================================

import { createStore } from './lib'
import { LS_THEME } from '@/constants'

// ── Types ────────────────────────────────────────────────────────────────────

type ThemeMode = 'light' | 'dark'

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
  theme: ThemeMode
  toasts: Toast[]
  commandPaletteOpen: boolean
}

// ── Safe localStorage ────────────────────────────────────────────────────────

const lsGet = (key: string): string | null => {
  try { return localStorage.getItem(key) } catch { return null }
}

const lsSet = (key: string, value: string): void => {
  try { localStorage.setItem(key, value) } catch { /* noop */ }
}

// ── Initialization helpers ───────────────────────────────────────────────────

const getSystemPreference = (): ThemeMode => {
  try {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  } catch {
    return 'light'
  }
}

const getInitialTheme = (): ThemeMode => {
  const stored = lsGet(LS_THEME)
  if (stored === 'light' || stored === 'dark') return stored
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

const selectTheme = (s: UIState): ThemeMode => s.theme

const selectToasts = (s: UIState): Toast[] => s.toasts

const selectCommandPaletteOpen = (s: UIState): boolean => s.commandPaletteOpen

// ── Theme ────────────────────────────────────────────────────────────────────

const setTheme = (mode: ThemeMode): void => {
  store.setState({ theme: mode })
  lsSet(LS_THEME, mode)
}

const toggleTheme = (): void => {
  const current = store.getState().theme
  setTheme(current === 'light' ? 'dark' : 'light')
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
      store.setState({ theme: e.matches ? 'dark' : 'light' })
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
  toggleTheme,
  addToast,
  dismissToast,
  openCommandPalette,
  closeCommandPalette,
  toggleCommandPalette,
  initSystemThemeListener,
}

export type { UIState, ThemeMode, Toast, ToastType, AddToastOptions }
