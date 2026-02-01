import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { AgentDetailPage } from './AgentDetailPage'

describe('AgentDetailPage', () => {
  it('renders agent detail with id from params', () => {
    render(
      <MemoryRouter initialEntries={['/agents/test-agent-id']}>
        <Routes>
          <Route path="/agents/:id" element={<AgentDetailPage />} />
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('Agent: test-agent-id')).toBeInTheDocument()
  })

  it('displays agent id from route params', () => {
    render(
      <MemoryRouter initialEntries={['/agents/agent-123']}>
        <Routes>
          <Route path="/agents/:id" element={<AgentDetailPage />} />
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('Agent: agent-123')).toBeInTheDocument()
  })
})
