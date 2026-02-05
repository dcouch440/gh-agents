import { render, screen } from '@testing-library/react'
import { ChatPage } from './ChatPage'

describe('ChatPage', () => {
  it('renders chat heading', () => {
    render(<ChatPage />)

    expect(screen.getByText('Chat')).toBeInTheDocument()
  })
})
