import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { AuthGuard } from './AuthGuard'
import { authStore } from '@/stores'

beforeEach(() => {
  authStore.store.setState({
    user: null,
    token: null,
    status: 'idle',
    error: null,
  })
})

const renderWithGuard = (initialEntries: string[] = ['/']) =>
  render(
    <MemoryRouter initialEntries={initialEntries}>
      <Routes>
        <Route element={<AuthGuard />}>
          <Route path="/" element={<div>Protected Content</div>} />
          <Route path="/settings" element={<div>Settings Page</div>} />
        </Route>
        <Route path="/login" element={<div>Login Page</div>} />
      </Routes>
    </MemoryRouter>,
  )

describe('AuthGuard', () => {
  it('shows loading spinner when status is idle', () => {
    authStore.store.setState({ status: 'idle' })

    renderWithGuard()

    expect(screen.getByText('Loading...')).toBeInTheDocument()
    expect(screen.queryByText('Protected Content')).not.toBeInTheDocument()
  })

  it('shows loading spinner when status is loading', () => {
    authStore.store.setState({ status: 'loading' })

    renderWithGuard()

    expect(screen.getByText('Loading...')).toBeInTheDocument()
    expect(screen.queryByText('Protected Content')).not.toBeInTheDocument()
  })

  it('redirects to login when unauthenticated', () => {
    authStore.store.setState({ status: 'unauthenticated' })

    renderWithGuard()

    expect(screen.getByText('Login Page')).toBeInTheDocument()
    expect(screen.queryByText('Protected Content')).not.toBeInTheDocument()
  })

  it('renders protected content when authenticated', () => {
    authStore.store.setState({
      status: 'authenticated',
      user: { id: 'u1', email: 'a@b.com', github_login: null },
      token: 'tok-123',
    })

    renderWithGuard()

    expect(screen.getByText('Protected Content')).toBeInTheDocument()
    expect(screen.queryByText('Login Page')).not.toBeInTheDocument()
  })

  it('renders nested routes when authenticated', () => {
    authStore.store.setState({
      status: 'authenticated',
      user: { id: 'u1', email: 'a@b.com', github_login: null },
      token: 'tok-123',
    })

    renderWithGuard(['/settings'])

    expect(screen.getByText('Settings Page')).toBeInTheDocument()
  })
})
