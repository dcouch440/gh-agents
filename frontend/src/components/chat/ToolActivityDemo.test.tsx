import { render, screen } from '@testing-library/react'
import { ToolActivityDemo } from './ToolActivityDemo'

describe('ToolActivityDemo', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('renders user message', () => {
    vi.useFakeTimers()

    render(<ToolActivityDemo />)

    expect(screen.getByText(/Find the auth module and update the middleware/)).toBeInTheDocument()

    vi.useRealTimers()
  })

  it('starts animation cycle on mount', () => {
    vi.useFakeTimers()

    render(<ToolActivityDemo />)

    // Fast-forward to first script step
    vi.advanceTimersByTime(100)

    // Should have chat demo container rendered
    expect(document.querySelector('.chat-demo')).toBeInTheDocument()

    vi.useRealTimers()
  })

  it('cleans up timers on unmount', () => {
    vi.useFakeTimers()

    const { unmount } = render(<ToolActivityDemo />)

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
