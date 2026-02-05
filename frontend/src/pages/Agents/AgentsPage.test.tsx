import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { AgentsPage } from './AgentsPage'

const mockNavigate = vi.fn()
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom')
  return { ...actual, useNavigate: () => mockNavigate }
})

vi.mock('@/hooks/useAgents', () => ({
  useAgents: () => ({
    agents: [],
    loading: false,
    error: null,
  }),
}))

vi.mock('@/hooks/useSessions', () => ({
  useSessions: () => ({
    sessions: [],
    loading: false,
    error: null,
  }),
}))

vi.mock('@/api', () => ({
  api: {
    sessions: { create: vi.fn() },
  },
}))

describe('AgentsPage', () => {
  it('renders agents heading', () => {
    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('Agents')).toBeInTheDocument()
  })
})
