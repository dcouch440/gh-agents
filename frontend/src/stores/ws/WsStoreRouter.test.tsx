import { render } from '@testing-library/react'
import { WsStoreRouter } from './WsStoreRouter'
import { sessionStore } from '@/stores/sessionStore'
import { roomStore } from '@/stores/roomStore'

// ── Mocks ────────────────────────────────────────────────────────────────────

const mockUnsubSession = vi.hoisted(() => vi.fn())
const mockUnsubRoom = vi.hoisted(() => vi.fn())
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
    mockSubscribe
      .mockReturnValueOnce(mockUnsubSession)
      .mockReturnValueOnce(mockUnsubRoom)
  })

  it('subscribes to session and room topics on mount', () => {
    render(<WsStoreRouter />)

    expect(mockSubscribe).toHaveBeenCalledTimes(2)
    expect(mockSubscribe).toHaveBeenCalledWith('session', sessionStore.handleWsEvent)
    expect(mockSubscribe).toHaveBeenCalledWith('room', roomStore.handleWsEvent)
  })

  it('unsubscribes on unmount', () => {
    const { unmount } = render(<WsStoreRouter />)

    unmount()

    expect(mockUnsubSession).toHaveBeenCalledTimes(1)
    expect(mockUnsubRoom).toHaveBeenCalledTimes(1)
  })

  it('renders nothing', () => {
    const { container } = render(<WsStoreRouter />)
    expect(container.innerHTML).toBe('')
  })
})
