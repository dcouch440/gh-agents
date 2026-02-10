import { render } from '@testing-library/react'
import { WsStoreRouter } from './WsStoreRouter'
import { sessionStore } from '@/stores/sessionStore'
import { roomStore } from '@/stores/roomStore'
import { workflowExecutionStore } from '@/stores/workflowExecutionStore'
import { activityStore } from '@/stores/activity'

// ── Mocks ────────────────────────────────────────────────────────────────────

const mockUnsubs = vi.hoisted(() => Array.from({ length: 6 }, () => vi.fn()))
const mockSubscribe = vi.hoisted(() => vi.fn())

vi.mock('@/hooks/useWebSocket', () => ({
  useWebSocket: () => ({
    subscribe: mockSubscribe,
    subscribeRun: vi.fn(() => vi.fn()),
    send: vi.fn(),
    status: 'connected',
    latency: null,
  }),
}))

vi.mock('@/api', () => ({ api: {} }))

// ── Tests ────────────────────────────────────────────────────────────────────

describe('WsStoreRouter', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockUnsubs.forEach((fn) => fn.mockClear())
    mockSubscribe.mockReset()
    for (let i = 0; i < 6; i++) {
      mockSubscribe.mockReturnValueOnce(mockUnsubs[i])
    }
  })

  it('subscribes to all 3 topics for domain stores + flight recorder (6 total)', () => {
    render(<WsStoreRouter />)

    expect(mockSubscribe).toHaveBeenCalledTimes(6)

    // Domain store handlers
    expect(mockSubscribe).toHaveBeenCalledWith('session', sessionStore.handleWsEvent)
    expect(mockSubscribe).toHaveBeenCalledWith('room', roomStore.handleWsEvent)
    expect(mockSubscribe).toHaveBeenCalledWith('workflow', workflowExecutionStore.handleWsEvent)

    // Flight recorder handlers
    expect(mockSubscribe).toHaveBeenCalledWith('session', activityStore.handleWsEvent)
    expect(mockSubscribe).toHaveBeenCalledWith('room', activityStore.handleWsEvent)
    expect(mockSubscribe).toHaveBeenCalledWith('workflow', activityStore.handleWsEvent)
  })

  it('unsubscribes all 6 handlers on unmount', () => {
    const { unmount } = render(<WsStoreRouter />)

    unmount()

    mockUnsubs.forEach((fn) => {
      expect(fn).toHaveBeenCalledTimes(1)
    })
  })

  it('renders nothing', () => {
    const { container } = render(<WsStoreRouter />)
    expect(container.innerHTML).toBe('')
  })
})
