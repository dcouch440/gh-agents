import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { ChatSessionPage } from './ChatSessionPage'

describe('ChatSessionPage', () => {
  it('renders chat session with sessionId from params', () => {
    render(
      <MemoryRouter initialEntries={['/chat/test-session-id']}>
        <Routes>
          <Route path="/chat/:sessionId" element={<ChatSessionPage />} />
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('Chat Session: test-session-id')).toBeInTheDocument()
  })

  it('displays session id from route params', () => {
    render(
      <MemoryRouter initialEntries={['/chat/session-456']}>
        <Routes>
          <Route path="/chat/:sessionId" element={<ChatSessionPage />} />
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('Chat Session: session-456')).toBeInTheDocument()
  })
})
