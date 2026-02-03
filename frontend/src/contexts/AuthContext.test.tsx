import { render, screen, waitFor } from '@testing-library/react'
import { act } from 'react'
import { AuthProvider } from './AuthContext'
import { useAuth } from '@/hooks/useAuth'
import type { User } from '@/types/user'

// ── Mocks ────────────────────────────────────────────────────────────────────

const { mockGet, mockPost } = vi.hoisted(() => ({
  mockGet: vi.fn(),
  mockPost: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    get: mockGet,
    post: mockPost,
  },
}))

vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, LS_AUTH_TOKEN: 'test_auth_token' }
})

// ── Test consumer ────────────────────────────────────────────────────────────

function TestConsumer() {
  const { user, token, loading, login, register, logout } = useAuth()

  return (
    <div>
      {loading && <div>loading</div>}
      {token && <div data-testid="token">{token}</div>}
      {user && <div data-testid="user">{user.email}</div>}
      <button onClick={() => { void login('test@example.com', 'password') }}>login</button>
      <button onClick={() => { void register('new@example.com', 'password') }}>register</button>
      <button onClick={logout}>logout</button>
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

const mockUser: User = {
  id: 'user-001',
  email: 'test@example.com',
  github_login: null,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

describe('AuthContext', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  describe('AuthProvider', () => {
    it('initializes with no token and no user', () => {
      render(
        <AuthProvider>
          <TestConsumer />
        </AuthProvider>,
      )

      expect(screen.queryByTestId('token')).not.toBeInTheDocument()
      expect(screen.queryByTestId('user')).not.toBeInTheDocument()
      expect(screen.queryByText('loading')).not.toBeInTheDocument()
    })

    it('hydrates user from stored token on mount', async () => {
      localStorage.setItem('test_auth_token', 'stored-token-123')
      mockGet.mockResolvedValue(mockUser)

      render(
        <AuthProvider>
          <TestConsumer />
        </AuthProvider>,
      )

      expect(screen.getByText('loading')).toBeInTheDocument()

      await waitFor(() => {
        expect(screen.getByTestId('token')).toHaveTextContent('stored-token-123')
        expect(screen.getByTestId('user')).toHaveTextContent('test@example.com')
      })

      expect(mockGet).toHaveBeenCalledWith('/auth/me')
      expect(screen.queryByText('loading')).not.toBeInTheDocument()
    })

    it('clears token when hydration fails', async () => {
      localStorage.setItem('test_auth_token', 'invalid-token')
      mockGet.mockRejectedValue(new Error('Unauthorized'))

      render(
        <AuthProvider>
          <TestConsumer />
        </AuthProvider>,
      )

      expect(screen.getByText('loading')).toBeInTheDocument()

      await waitFor(() => {
        expect(screen.queryByText('loading')).not.toBeInTheDocument()
      })

      expect(screen.queryByTestId('token')).not.toBeInTheDocument()
      expect(screen.queryByTestId('user')).not.toBeInTheDocument()
      expect(localStorage.getItem('test_auth_token')).toBeNull()
    })

    it('logs in user and saves token', async () => {
      mockPost.mockResolvedValue({
        token: 'new-token-456',
        user: mockUser,
      })

      render(
        <AuthProvider>
          <TestConsumer />
        </AuthProvider>,
      )

      act(() => {
        screen.getByText('login').click()
      })

      await waitFor(() => {
        expect(screen.getByTestId('token')).toHaveTextContent('new-token-456')
        expect(screen.getByTestId('user')).toHaveTextContent('test@example.com')
      })

      expect(mockPost).toHaveBeenCalledWith('/auth/login', {
        email: 'test@example.com',
        password: 'password',
      })
      expect(localStorage.getItem('test_auth_token')).toBe('new-token-456')
    })

    it('registers user and saves token', async () => {
      const newUser: User = {
        id: 'user-002',
        email: 'new@example.com',
        github_login: null,
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-01T00:00:00Z',
      }

      mockPost.mockResolvedValue({
        token: 'register-token-789',
        user: newUser,
      })

      render(
        <AuthProvider>
          <TestConsumer />
        </AuthProvider>,
      )

      act(() => {
        screen.getByText('register').click()
      })

      await waitFor(() => {
        expect(screen.getByTestId('token')).toHaveTextContent('register-token-789')
        expect(screen.getByTestId('user')).toHaveTextContent('new@example.com')
      })

      expect(mockPost).toHaveBeenCalledWith('/auth/register', {
        email: 'new@example.com',
        password: 'password',
      })
      expect(localStorage.getItem('test_auth_token')).toBe('register-token-789')
    })

    it('logs out user and clears token', async () => {
      localStorage.setItem('test_auth_token', 'stored-token-123')
      mockGet.mockResolvedValue(mockUser)

      render(
        <AuthProvider>
          <TestConsumer />
        </AuthProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('token')).toBeInTheDocument()
      })

      act(() => {
        screen.getByText('logout').click()
      })

      expect(screen.queryByTestId('token')).not.toBeInTheDocument()
      expect(screen.queryByTestId('user')).not.toBeInTheDocument()
      expect(localStorage.getItem('test_auth_token')).toBeNull()
    })

    it('throws when useAuth is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useAuth must be used within AuthProvider')
      spy.mockRestore()
    })
  })
})
