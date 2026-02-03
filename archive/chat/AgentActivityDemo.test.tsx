import { render, screen } from '@testing-library/react'
import { AgentActivityDemo } from './AgentActivityDemo'

describe('AgentActivityDemo', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('renders user message', () => {
    vi.useFakeTimers()

    render(<AgentActivityDemo />)

    expect(screen.getByText(/find the auth module and update the middleware/i)).toBeInTheDocument()

    vi.useRealTimers()
  })

  it('starts animation cycle on mount', () => {
    vi.useFakeTimers()

    render(<AgentActivityDemo />)

    // Fast-forward to first script step
    vi.advanceTimersByTime(100)

    // Should have at least one agent activity module rendered
    expect(document.querySelector('.activity-demo')).toBeInTheDocument()

    vi.useRealTimers()
  })

  it('cleans up timers on unmount', () => {
    vi.useFakeTimers()

    const { unmount } = render(<AgentActivityDemo />)

    // Create some timers
    vi.advanceTimersByTime(100)

    // Should have pending timers
    expect(vi.getTimerCount()).toBeGreaterThan(0)

    unmount()

    // Cleanup should have cleared timers
    expect(vi.getTimerCount()).toBe(0)

    vi.useRealTimers()
  })
})
