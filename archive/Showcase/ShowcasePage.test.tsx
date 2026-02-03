import { render, screen } from '@testing-library/react'
import { ShowcasePage } from './ShowcasePage'

describe('ShowcasePage', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('renders showcase header', () => {
    vi.useFakeTimers()

    render(<ShowcasePage />)

    expect(screen.getByText('COMPONENT SHOWCASE')).toBeInTheDocument()

    vi.useRealTimers()
  })

  it('renders all showcase sections', () => {
    vi.useFakeTimers()

    render(<ShowcasePage />)

    expect(screen.getByText('AGENT POOL')).toBeInTheDocument()
    expect(screen.getByText('SYSTEM HEALTH')).toBeInTheDocument()
    expect(screen.getByText('TASK QUEUE')).toBeInTheDocument()
    expect(screen.getByText('TOKEN USAGE (24h)')).toBeInTheDocument()
    expect(screen.getByText('PIPELINE')).toBeInTheDocument()
    expect(screen.getByText('ACTIVITY FEED')).toBeInTheDocument()
    expect(screen.getByText('CHAT ACTIVITY')).toBeInTheDocument()

    vi.useRealTimers()
  })

  it('cleans up timers on unmount', () => {
    vi.useFakeTimers()

    const { unmount } = render(<ShowcasePage />)

    // Advance to create some timers
    vi.advanceTimersByTime(100)

    expect(vi.getTimerCount()).toBeGreaterThan(0)

    unmount()

    expect(vi.getTimerCount()).toBe(0)

    vi.useRealTimers()
  })
})
