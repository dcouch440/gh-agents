import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { LoginPage } from './LoginPage'
import { authStore } from '@/stores'

beforeEach(() => {
  vi.clearAllMocks()
  authStore.store.setState({
    user: null,
    token: null,
    status: 'unauthenticated',
    error: null,
  })
})

const renderLogin = (initialEntries: string[] = ['/login']) =>
  render(
    <MemoryRouter initialEntries={initialEntries}>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="/" element={<div>Dashboard</div>} />
      </Routes>
    </MemoryRouter>,
  )

describe('LoginPage', () => {
  it('renders login form with email and password fields', () => {
    renderLogin()
    expect(screen.getByText(/Login to/)).toBeInTheDocument()
    expect(screen.getByLabelText(/Email/)).toBeInTheDocument()
    expect(screen.getByLabelText(/Password/)).toBeInTheDocument()
    expect(screen.getByText('Login')).toBeInTheDocument()
  })

  it('redirects to dashboard when user is already authenticated', () => {
    authStore.store.setState({
      user: { id: 'u1', email: 'test@test.com', github_login: null },
      token: 'fake-token',
      status: 'authenticated',
      error: null,
    })

    renderLogin()
    expect(screen.getByText('Dashboard')).toBeInTheDocument()
  })

  it('allows typing in email and password fields', async () => {
    const user = userEvent.setup()
    renderLogin()

    const emailField = screen.getByLabelText(/Email/)
    const passwordField = screen.getByLabelText(/Password/)

    await user.type(emailField, 'user@example.com')
    await user.type(passwordField, 'password123')

    expect(emailField).toHaveValue('user@example.com')
    expect(passwordField).toHaveValue('password123')
  })

  it('calls authStore.login on form submit', async () => {
    const user = userEvent.setup()
    const loginSpy = vi.spyOn(authStore, 'login').mockResolvedValue(undefined)

    renderLogin()

    await user.type(screen.getByLabelText(/Email/), 'user@example.com')
    await user.type(screen.getByLabelText(/Password/), 'password123')
    await user.click(screen.getByText('Login'))

    expect(loginSpy).toHaveBeenCalledWith('user@example.com', 'password123')
  })

  it('shows error message when login fails', async () => {
    const user = userEvent.setup()
    vi.spyOn(authStore, 'login').mockRejectedValue(new Error('Invalid credentials'))

    renderLogin()

    await user.type(screen.getByLabelText(/Email/), 'user@example.com')
    await user.type(screen.getByLabelText(/Password/), 'wrong')
    await user.click(screen.getByText('Login'))

    expect(await screen.findByText('Invalid credentials')).toBeInTheDocument()
  })

  it('shows "Logging in..." text when submitting', async () => {
    const user = userEvent.setup()
    // Never resolves during test — keeps loading state
    vi.spyOn(authStore, 'login').mockReturnValue(new Promise(() => {}))

    renderLogin()

    await user.type(screen.getByLabelText(/Email/), 'user@example.com')
    await user.type(screen.getByLabelText(/Password/), 'pass')
    await user.click(screen.getByText('Login'))

    expect(await screen.findByText('Logging in...')).toBeInTheDocument()
  })
})
