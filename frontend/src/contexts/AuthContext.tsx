import { createContext, useState, useCallback, useEffect, type ReactNode } from 'react'
import { api } from '@/api'
import { LS_AUTH_TOKEN } from '@/constants'
import type { User } from '@/types/user'

type AuthState = {
  user: User | null
  token: string | null
  loading: boolean
  login: (email: string, password: string) => Promise<void>
  register: (email: string, password: string) => Promise<void>
  logout: () => void
}

const AuthContext = createContext<AuthState | null>(null)

const hasStoredToken = () => localStorage.getItem(LS_AUTH_TOKEN) !== null

function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [token, setToken] = useState<string | null>(
    () => localStorage.getItem(LS_AUTH_TOKEN),
  )
  const [loading, setLoading] = useState(hasStoredToken)

  const saveToken = useCallback((t: string) => {
    localStorage.setItem(LS_AUTH_TOKEN, t)
    setToken(t)
  }, [])

  const clearToken = useCallback(() => {
    localStorage.removeItem(LS_AUTH_TOKEN)
    setToken(null)
    setUser(null)
  }, [])

  const login = useCallback(async (email: string, password: string) => {
    const res = await api.auth.login({ email, password })
    saveToken(res.token)
    const meResponse = await api.auth.me()
    setUser({
      id: meResponse.id,
      email: meResponse.email,
      github_login: meResponse.github_login,
      created_at: '',
      updated_at: '',
    })
  }, [saveToken])

  const register = useCallback(async (email: string, password: string) => {
    const res = await api.auth.register({ email, password })
    saveToken(res.token)
    setUser({
      id: res.user.id,
      email: res.user.email,
      github_login: res.user.github_login,
      created_at: '',
      updated_at: '',
    })
  }, [saveToken])

  const logout = useCallback(() => {
    clearToken()
  }, [clearToken])

  // Hydrate user from token on mount
  useEffect(() => {
    if (!token) return

    let cancelled = false
    api.auth.me()
      .then((meResponse) => {
        if (!cancelled) {
          setUser({
            id: meResponse.id,
            email: meResponse.email,
            github_login: meResponse.github_login,
            created_at: '',
            updated_at: '',
          })
        }
      })
      .catch(() => { if (!cancelled) clearToken() })
      .finally(() => { if (!cancelled) setLoading(false) })

    return () => { cancelled = true }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <AuthContext.Provider value={{ user, token, loading, login, register, logout }}>
      {children}
    </AuthContext.Provider>
  )
}

export { AuthContext, AuthProvider }
export type { AuthState }
