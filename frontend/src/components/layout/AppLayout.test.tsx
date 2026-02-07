import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { AppLayout } from './AppLayout'

const mockUseAuth = vi.hoisted(() =>
  vi.fn(() => ({
    user: { id: 'u1', email: 'test@test.com', github_login: null, created_at: '', updated_at: '' },
    token: 'fake-token',
    loading: false,
    login: vi.fn(),
    register: vi.fn(),
    logout: vi.fn(),
  })),
)

vi.mock('@/hooks/useAuth', () => ({
  useAuth: mockUseAuth,
}))

vi.mock('./Sidebar', () => ({
  Sidebar: function Sidebar() {
    return <nav data-testid="sidebar">nexor</nav>
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
})

describe('AppLayout', () => {
  it('renders sidebar and outlet', () => {
    render(
      <MemoryRouter initialEntries={['/']}>
        <Routes>
          <Route path="/" element={<AppLayout />}>
            <Route index element={<div>Test Page Content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('nexor')).toBeInTheDocument()
    expect(screen.getByText('Test Page Content')).toBeInTheDocument()
  })

  it('renders multiple routes through outlet', () => {
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

    expect(screen.getByText('nexor')).toBeInTheDocument()
    expect(screen.getByText('Home')).toBeInTheDocument()
  })
})
