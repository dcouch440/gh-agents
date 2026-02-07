import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { AppLayout } from './AppLayout'
import { authStore } from '@/stores'

vi.mock('./ThemeToggle', () => ({
  ThemeToggle: function ThemeToggle() {
    return <button data-testid="theme-toggle">toggle</button>
  },
}))

vi.mock('./TopNavBar', () => ({
  TopNavBar: function TopNavBar() {
    return <nav data-testid="top-nav-bar">nav</nav>
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
})

describe('AppLayout', () => {
  it('renders top nav bar and outlet when authenticated', () => {
    authStore.store.setState({
      user: { id: 'u1', email: 'test@test.com', github_login: null },
      token: 'fake-token',
      status: 'authenticated',
      error: null,
    })

    render(
      <MemoryRouter initialEntries={['/']}>
        <Routes>
          <Route path="/" element={<AppLayout />}>
            <Route index element={<div>Test Page Content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByTestId('top-nav-bar')).toBeInTheDocument()
    expect(screen.getByText('Test Page Content')).toBeInTheDocument()
  })

  it('renders multiple routes through outlet', () => {
    authStore.store.setState({
      user: { id: 'u1', email: 'test@test.com', github_login: null },
      token: 'fake-token',
      status: 'authenticated',
      error: null,
    })

    render(
      <MemoryRouter initialEntries={['/']}>
        <Routes>
          <Route path="/" element={<AppLayout />}>
            <Route index element={<div>Home</div>} />
            <Route path="about" element={<div>About</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('Home')).toBeInTheDocument()
  })

  it('shows loading spinner when status is idle', () => {
    authStore.store.setState({
      user: null,
      token: null,
      status: 'idle',
      error: null,
    })

    render(
      <MemoryRouter initialEntries={['/']}>
        <Routes>
          <Route path="/" element={<AppLayout />}>
            <Route index element={<div>Home</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('Loading...')).toBeInTheDocument()
    expect(screen.queryByText('Home')).not.toBeInTheDocument()
  })

  it('redirects to login when unauthenticated', () => {
    authStore.store.setState({
      user: null,
      token: null,
      status: 'unauthenticated',
      error: null,
    })

    render(
      <MemoryRouter initialEntries={['/']}>
        <Routes>
          <Route path="/" element={<AppLayout />}>
            <Route index element={<div>Home</div>} />
          </Route>
          <Route path="/login" element={<div>Login Page</div>} />
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('Login Page')).toBeInTheDocument()
    expect(screen.queryByText('Home')).not.toBeInTheDocument()
  })
})
