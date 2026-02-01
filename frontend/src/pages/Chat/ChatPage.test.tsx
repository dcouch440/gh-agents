import { render } from '@testing-library/react'
import { ChatPage } from './ChatPage'

describe('ChatPage', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('renders AgentActivityDemo component', () => {
    vi.useFakeTimers()

    const { container } = render(<ChatPage />)

    expect(container.querySelector('.activity-demo')).toBeInTheDocument()

    vi.useRealTimers()
  })
})
