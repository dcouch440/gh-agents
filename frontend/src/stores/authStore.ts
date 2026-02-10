// ============================================================================
// authStore — Authentication State
// ============================================================================

import { createStore } from './lib'
import { api } from '@/api'
import { LS_AUTH_TOKEN } from '@/constants'

// ── Types ────────────────────────────────────────────────────────────────────

type User = {
  id: string
  email: string
  github_login: string | null
}

type AuthStatus = 'idle' | 'loading' | 'authenticated' | 'unauthenticated'

type AuthState = {
  user: User | null
  token: string | null
  status: AuthStatus
  error: string | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<AuthState>(() => ({
  user: null,
  token: null,
  status: 'idle',
  error: null,
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

const extractError = (e: unknown): string => (e instanceof Error ? e.message : 'Unknown error')

const saveToken = (token: string): void => {
  localStorage.setItem(LS_AUTH_TOKEN, token)
}

const clearToken = (): void => {
  localStorage.removeItem(LS_AUTH_TOKEN)
}

// ── Actions ──────────────────────────────────────────────────────────────────

const login = async (email: string, password: string): Promise<void> => {
  store.setState({ status: 'loading', error: null })
  try {
    const res = await api.auth.login({ email, password })
    saveToken(res.token)
    const me = await api.auth.me()
    store.setState({
      user: { id: me.id, email: me.email, github_login: me.github_login },
      token: res.token,
      status: 'authenticated',
    })
  } catch (e) {
    store.setState({ status: 'unauthenticated', error: extractError(e) })
    throw e
  }
}

const register = async (email: string, password: string): Promise<void> => {
  store.setState({ status: 'loading', error: null })
  try {
    const res = await api.auth.register({ email, password })
    saveToken(res.token)
    store.setState({
      user: { id: res.user.id, email: res.user.email, github_login: res.user.github_login },
      token: res.token,
      status: 'authenticated',
    })
  } catch (e) {
    store.setState({ status: 'unauthenticated', error: extractError(e) })
    throw e
  }
}

const logout = (): void => {
  clearToken()
  store.setState({ user: null, token: null, status: 'unauthenticated', error: null })
}

const hydrate = async (): Promise<void> => {
  const token = localStorage.getItem(LS_AUTH_TOKEN)
  if (!token) {
    store.setState({ status: 'unauthenticated' })
    return
  }

  store.setState({ status: 'loading', error: null })
  try {
    const me = await api.auth.me()
    store.setState({
      user: { id: me.id, email: me.email, github_login: me.github_login },
      token,
      status: 'authenticated',
    })
  } catch {
    clearToken()
    store.setState({ user: null, token: null, status: 'unauthenticated' })
  }
}

// ── Selectors ────────────────────────────────────────────────────────────────

const selectUser = (s: AuthState): User | null => s.user
const selectIsAuthenticated = (s: AuthState): boolean => s.status === 'authenticated'
const selectAuthStatus = (s: AuthState): AuthStatus => s.status
const selectAuthError = (s: AuthState): string | null => s.error

// ── Exports ──────────────────────────────────────────────────────────────────

const authStore = { store, login, register, logout, hydrate }

export { authStore, selectUser, selectIsAuthenticated, selectAuthStatus, selectAuthError }
export type { AuthState, AuthStatus, User }
