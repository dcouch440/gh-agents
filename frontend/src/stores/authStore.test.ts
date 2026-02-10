import { LS_AUTH_TOKEN } from '@/constants'
import { authStore, selectUser, selectIsAuthenticated, selectAuthStatus, selectAuthError } from './authStore'

// ── Mocks ────────────────────────────────────────────────────────────────────

const { mockLogin, mockRegister, mockMe } = vi.hoisted(() => ({
  mockLogin: vi.fn(),
  mockRegister: vi.fn(),
  mockMe: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    auth: {
      login: mockLogin,
      register: mockRegister,
      me: mockMe,
    },
  },
}))

const mockStorage = vi.hoisted(() => {
  const store = new Map<string, string>()
  return {
    store,
    getItem: vi.fn((key: string) => store.get(key) ?? null),
    setItem: vi.fn((key: string, value: string) => {
      store.set(key, value)
    }),
    removeItem: vi.fn((key: string) => {
      store.delete(key)
    }),
    clear: vi.fn(() => {
      store.clear()
    }),
    get length() {
      return store.size
    },
    key: vi.fn(() => null),
  }
})

// ── Setup ────────────────────────────────────────────────────────────────────

const resetStore = () => {
  mockStorage.store.clear()
  authStore.store.setState({
    user: null,
    token: null,
    status: 'idle',
    error: null,
  })
}

beforeEach(() => {
  vi.clearAllMocks()
  Object.defineProperty(window, 'localStorage', {
    value: mockStorage,
    writable: true,
  })
  resetStore()
})

// ── Tests ────────────────────────────────────────────────────────────────────

describe('authStore', () => {
  describe('login', () => {
    it('sets status to authenticated on success', async () => {
      mockLogin.mockResolvedValue({ token: 'tok-123', expires_in: 3600 })
      mockMe.mockResolvedValue({ id: 'u1', email: 'a@b.com', github_login: null })

      await authStore.login('a@b.com', 'pass')

      const state = authStore.store.getState()
      expect(state.status).toBe('authenticated')
      expect(state.user).toEqual({ id: 'u1', email: 'a@b.com', github_login: null })
      expect(state.token).toBe('tok-123')
      expect(state.error).toBeNull()
    })

    it('saves token to localStorage', async () => {
      mockLogin.mockResolvedValue({ token: 'tok-456', expires_in: 3600 })
      mockMe.mockResolvedValue({ id: 'u1', email: 'a@b.com', github_login: null })

      await authStore.login('a@b.com', 'pass')

      expect(mockStorage.setItem).toHaveBeenCalledWith(LS_AUTH_TOKEN, 'tok-456')
      expect(mockStorage.store.get(LS_AUTH_TOKEN)).toBe('tok-456')
    })

    it('sets loading status during login', async () => {
      const statuses: string[] = []
      const unsub = authStore.store.subscribe(() => {
        statuses.push(authStore.store.getState().status)
      })

      mockLogin.mockResolvedValue({ token: 'tok', expires_in: 3600 })
      mockMe.mockResolvedValue({ id: 'u1', email: 'a@b.com', github_login: null })

      await authStore.login('a@b.com', 'pass')
      unsub()

      expect(statuses[0]).toBe('loading')
      expect(statuses[statuses.length - 1]).toBe('authenticated')
    })

    it('sets error and unauthenticated on failure', async () => {
      mockLogin.mockRejectedValue(new Error('Invalid credentials'))

      await expect(authStore.login('a@b.com', 'bad')).rejects.toThrow('Invalid credentials')

      const state = authStore.store.getState()
      expect(state.status).toBe('unauthenticated')
      expect(state.error).toBe('Invalid credentials')
      expect(state.user).toBeNull()
    })

    it('throws on failure so caller can handle', async () => {
      mockLogin.mockRejectedValue(new Error('Network error'))

      await expect(authStore.login('a@b.com', 'pass')).rejects.toThrow('Network error')
    })
  })

  describe('register', () => {
    it('sets user and token from response', async () => {
      mockRegister.mockResolvedValue({
        token: 'reg-tok',
        expires_in: 3600,
        user: { id: 'u2', email: 'new@b.com', github_login: 'gh-user' },
      })

      await authStore.register('new@b.com', 'pass')

      const state = authStore.store.getState()
      expect(state.status).toBe('authenticated')
      expect(state.user).toEqual({ id: 'u2', email: 'new@b.com', github_login: 'gh-user' })
      expect(state.token).toBe('reg-tok')
    })

    it('saves token to localStorage', async () => {
      mockRegister.mockResolvedValue({
        token: 'reg-tok-2',
        expires_in: 3600,
        user: { id: 'u2', email: 'new@b.com', github_login: null },
      })

      await authStore.register('new@b.com', 'pass')

      expect(mockStorage.setItem).toHaveBeenCalledWith(LS_AUTH_TOKEN, 'reg-tok-2')
    })

    it('sets error on failure', async () => {
      mockRegister.mockRejectedValue(new Error('Email taken'))

      await expect(authStore.register('taken@b.com', 'pass')).rejects.toThrow('Email taken')

      const state = authStore.store.getState()
      expect(state.status).toBe('unauthenticated')
      expect(state.error).toBe('Email taken')
    })
  })

  describe('logout', () => {
    it('clears user, token, and sets unauthenticated', async () => {
      mockLogin.mockResolvedValue({ token: 'tok', expires_in: 3600 })
      mockMe.mockResolvedValue({ id: 'u1', email: 'a@b.com', github_login: null })
      await authStore.login('a@b.com', 'pass')

      authStore.logout()

      const state = authStore.store.getState()
      expect(state.user).toBeNull()
      expect(state.token).toBeNull()
      expect(state.status).toBe('unauthenticated')
      expect(state.error).toBeNull()
    })

    it('removes token from localStorage', () => {
      mockStorage.store.set(LS_AUTH_TOKEN, 'some-token')

      authStore.logout()

      expect(mockStorage.removeItem).toHaveBeenCalledWith(LS_AUTH_TOKEN)
      expect(mockStorage.store.has(LS_AUTH_TOKEN)).toBe(false)
    })
  })

  describe('hydrate', () => {
    it('restores session from localStorage token', async () => {
      mockStorage.store.set(LS_AUTH_TOKEN, 'stored-tok')
      mockMe.mockResolvedValue({ id: 'u1', email: 'a@b.com', github_login: 'gh' })

      await authStore.hydrate()

      const state = authStore.store.getState()
      expect(state.status).toBe('authenticated')
      expect(state.user).toEqual({ id: 'u1', email: 'a@b.com', github_login: 'gh' })
      expect(state.token).toBe('stored-tok')
    })

    it('sets unauthenticated and clears token if hydration fails', async () => {
      mockStorage.store.set(LS_AUTH_TOKEN, 'expired-tok')
      mockMe.mockRejectedValue(new Error('Token expired'))

      await authStore.hydrate()

      const state = authStore.store.getState()
      expect(state.status).toBe('unauthenticated')
      expect(state.user).toBeNull()
      expect(state.token).toBeNull()
      expect(mockStorage.removeItem).toHaveBeenCalledWith(LS_AUTH_TOKEN)
    })

    it('sets unauthenticated if no token in localStorage', async () => {
      await authStore.hydrate()

      const state = authStore.store.getState()
      expect(state.status).toBe('unauthenticated')
      expect(mockMe).not.toHaveBeenCalled()
    })
  })

  describe('selectors', () => {
    it('selectUser returns user', () => {
      const user = { id: 'u1', email: 'a@b.com', github_login: null }
      authStore.store.setState({ user })
      expect(selectUser(authStore.store.getState())).toEqual(user)
    })

    it('selectIsAuthenticated returns true only when authenticated', () => {
      authStore.store.setState({ status: 'idle' })
      expect(selectIsAuthenticated(authStore.store.getState())).toBe(false)

      authStore.store.setState({ status: 'loading' })
      expect(selectIsAuthenticated(authStore.store.getState())).toBe(false)

      authStore.store.setState({ status: 'unauthenticated' })
      expect(selectIsAuthenticated(authStore.store.getState())).toBe(false)

      authStore.store.setState({ status: 'authenticated' })
      expect(selectIsAuthenticated(authStore.store.getState())).toBe(true)
    })

    it('selectAuthStatus returns current status', () => {
      authStore.store.setState({ status: 'loading' })
      expect(selectAuthStatus(authStore.store.getState())).toBe('loading')
    })

    it('selectAuthError returns error', () => {
      authStore.store.setState({ error: 'Something went wrong' })
      expect(selectAuthError(authStore.store.getState())).toBe('Something went wrong')
    })
  })
})
