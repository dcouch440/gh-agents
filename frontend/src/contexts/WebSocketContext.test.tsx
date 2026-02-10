import { renderHook, act } from '@testing-library/react'
import { useContext } from 'react'
import type { ReactNode } from 'react'
import { WebSocketContext, WebSocketProvider } from './WebSocketContext'

// ── Hoisted refs ─────────────────────────────────────────────────────────────

const mockToken = vi.hoisted(() => ({ current: null as string | null }))
const mockWsSetState = vi.hoisted(() => vi.fn())

// ── Module mocks ─────────────────────────────────────────────────────────────

vi.mock('@/stores/lib', () => ({
  useStore: (_store: unknown, selector: (s: { token: string | null }) => unknown) => selector({ token: mockToken.current }),
}))

vi.mock('@/stores/authStore', () => ({
  authStore: { store: {} },
}))

vi.mock('@/stores/wsConnectionStore', () => ({
  wsConnectionStore: { setState: mockWsSetState },
}))

// ── Mock WebSocket ───────────────────────────────────────────────────────────

type MockWsInstance = {
  url: string
  readyState: number
  send: ReturnType<typeof vi.fn>
  close: ReturnType<typeof vi.fn>
  onopen: ((ev: Event) => void) | null
  onmessage: ((ev: MessageEvent) => void) | null
  onclose: ((ev: CloseEvent) => void) | null
  onerror: ((ev: Event) => void) | null
}

let lastWs: MockWsInstance | null = null

class FakeWebSocket {
  static OPEN = 1
  static CLOSED = 3
  static CONNECTING = 0
  static CLOSING = 2

  url: string
  readyState = 1
  send = vi.fn()
  close = vi.fn()
  onopen: ((ev: Event) => void) | null = null
  onmessage: ((ev: MessageEvent) => void) | null = null
  onclose: ((ev: CloseEvent) => void) | null = null
  onerror: ((ev: Event) => void) | null = null

  constructor(url: string) {
    this.url = url
    lastWs = this as unknown as MockWsInstance
  }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

const wrapper = ({ children }: { children: ReactNode }) => <WebSocketProvider>{children}</WebSocketProvider>

const renderProvider = () => renderHook(() => useContext(WebSocketContext), { wrapper })

// ── Tests ────────────────────────────────────────────────────────────────────

describe('WebSocketContext', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.useFakeTimers()
    lastWs = null
    mockToken.current = null
    Object.defineProperty(globalThis, 'WebSocket', { value: FakeWebSocket, writable: true })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('provides context value when wrapped in provider', () => {
    const { result } = renderProvider()

    expect(result.current).not.toBeNull()
    expect(result.current!.subscribe).toBeInstanceOf(Function)
    expect(result.current!.subscribeRun).toBeInstanceOf(Function)
    expect(result.current!.send).toBeInstanceOf(Function)
  })

  it('does not connect when token is null', () => {
    mockToken.current = null
    renderProvider()

    expect(lastWs).toBeNull()
    expect(mockWsSetState).toHaveBeenCalledWith({
      status: 'disconnected',
      latency: null,
    })
  })

  it('connects WebSocket when token is present', () => {
    mockToken.current = 'test-jwt'
    renderProvider()

    expect(lastWs).not.toBeNull()
    expect(lastWs!.url).toContain('token=test-jwt')
    expect(mockWsSetState).toHaveBeenCalledWith({ status: 'connecting' })
  })

  it('sets status to connected on open', () => {
    mockToken.current = 'test-jwt'
    renderProvider()

    act(() => {
      lastWs!.onopen!(new Event('open'))
    })

    expect(mockWsSetState).toHaveBeenCalledWith({ status: 'connected' })
  })

  it('dispatches broadcast messages to subscribed handlers', () => {
    mockToken.current = 'test-jwt'
    const { result } = renderProvider()

    act(() => {
      lastWs!.onopen!(new Event('open'))
    })

    const handler = vi.fn()
    act(() => {
      result.current!.subscribe('session', handler)
    })

    const wireMsg = JSON.stringify({
      topic: 'session',
      event: 'created',
      ts: '2025-01-01T00:00:00Z',
      run_id: null,
      user_id: null,
      data: { session_id: 's1' },
    })

    act(() => {
      lastWs!.onmessage!(new MessageEvent('message', { data: wireMsg }))
    })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler).toHaveBeenCalledWith(expect.objectContaining({ topic: 'session', event: 'created' }))
  })

  it('does not dispatch to unsubscribed handlers', () => {
    mockToken.current = 'test-jwt'
    const { result } = renderProvider()

    act(() => {
      lastWs!.onopen!(new Event('open'))
    })

    const handler = vi.fn()
    let unsub!: () => void
    act(() => {
      unsub = result.current!.subscribe('session', handler)
    })

    act(() => {
      unsub()
    })

    const wireMsg = JSON.stringify({
      topic: 'session',
      event: 'created',
      ts: '2025-01-01T00:00:00Z',
      run_id: null,
      user_id: null,
      data: {},
    })

    act(() => {
      lastWs!.onmessage!(new MessageEvent('message', { data: wireMsg }))
    })

    expect(handler).not.toHaveBeenCalled()
  })

  it('sends SUBSCRIBE when first handler registers for a topic', () => {
    mockToken.current = 'test-jwt'
    const { result } = renderProvider()

    act(() => {
      lastWs!.onopen!(new Event('open'))
    })

    lastWs!.send.mockClear()

    act(() => {
      result.current!.subscribe('session', vi.fn())
    })

    expect(lastWs!.send).toHaveBeenCalledWith(JSON.stringify({ type: 'subscribe', topics: ['session'] }))
  })

  it('sends UNSUBSCRIBE when last handler for a topic unregisters', () => {
    mockToken.current = 'test-jwt'
    const { result } = renderProvider()

    act(() => {
      lastWs!.onopen!(new Event('open'))
    })

    let unsub!: () => void
    act(() => {
      unsub = result.current!.subscribe('room', vi.fn())
    })

    lastWs!.send.mockClear()

    act(() => {
      unsub()
    })

    expect(lastWs!.send).toHaveBeenCalledWith(JSON.stringify({ type: 'unsubscribe', topics: ['room'] }))
  })

  it('sets status to disconnected and schedules reconnect on close', () => {
    mockToken.current = 'test-jwt'
    renderProvider()

    act(() => {
      lastWs!.onopen!(new Event('open'))
    })

    mockWsSetState.mockClear()
    const firstWs = lastWs

    act(() => {
      lastWs!.onclose!(new CloseEvent('close'))
    })

    expect(mockWsSetState).toHaveBeenCalledWith({
      status: 'disconnected',
      latency: null,
    })

    // After reconnect delay, a new WebSocket should be created
    act(() => {
      vi.advanceTimersByTime(2000)
    })

    expect(lastWs).not.toBe(firstWs)
  })

  it('re-subscribes active topics on reconnect', () => {
    mockToken.current = 'test-jwt'
    const { result } = renderProvider()

    act(() => {
      lastWs!.onopen!(new Event('open'))
    })

    // Subscribe to a topic
    act(() => {
      result.current!.subscribe('session', vi.fn())
    })

    // Close and reconnect
    act(() => {
      lastWs!.onclose!(new CloseEvent('close'))
    })

    act(() => {
      vi.advanceTimersByTime(2000)
    })

    // Fire onopen on the new WebSocket
    lastWs!.send.mockClear()
    act(() => {
      lastWs!.onopen!(new Event('open'))
    })

    // Should re-subscribe to 'session'
    expect(lastWs!.send).toHaveBeenCalledWith(JSON.stringify({ type: 'subscribe', topics: ['session'] }))
  })

  it('handles control messages (pong) and updates latency', () => {
    mockToken.current = 'test-jwt'
    renderProvider()

    act(() => {
      lastWs!.onopen!(new Event('open'))
    })

    mockWsSetState.mockClear()

    const controlMsg = JSON.stringify({
      type: 'pong',
      client_ts: new Date(Date.now() - 42).toISOString(),
      server_ts: new Date().toISOString(),
    })

    act(() => {
      lastWs!.onmessage!(new MessageEvent('message', { data: controlMsg }))
    })

    const latencyCall = mockWsSetState.mock.calls.find((call: unknown[]) => (call[0] as Record<string, unknown>).latency !== undefined)
    expect(latencyCall).toBeDefined()
    expect(typeof (latencyCall![0] as Record<string, unknown>).latency).toBe('number')
  })

  it('subscribeRun sends SUBSCRIBE_RUN and returns cleanup', () => {
    mockToken.current = 'test-jwt'
    const { result } = renderProvider()

    act(() => {
      lastWs!.onopen!(new Event('open'))
    })

    lastWs!.send.mockClear()

    let unsub!: () => void
    act(() => {
      unsub = result.current!.subscribeRun('run-123')
    })

    expect(lastWs!.send).toHaveBeenCalledWith(JSON.stringify({ type: 'subscribe_run', run_id: 'run-123' }))

    lastWs!.send.mockClear()

    act(() => {
      unsub()
    })

    expect(lastWs!.send).toHaveBeenCalledWith(JSON.stringify({ type: 'unsubscribe_run', run_id: 'run-123' }))
  })

  it('ignores non-JSON messages', () => {
    mockToken.current = 'test-jwt'
    renderProvider()

    act(() => {
      lastWs!.onopen!(new Event('open'))
    })

    // Should not throw
    act(() => {
      lastWs!.onmessage!(new MessageEvent('message', { data: 'not json' }))
    })
  })

  it('isolates handler errors from other handlers', () => {
    mockToken.current = 'test-jwt'
    const { result } = renderProvider()

    act(() => {
      lastWs!.onopen!(new Event('open'))
    })

    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const badHandler = vi.fn(() => {
      throw new Error('handler boom')
    })
    const goodHandler = vi.fn()

    act(() => {
      result.current!.subscribe('session', badHandler)
      result.current!.subscribe('session', goodHandler)
    })

    const wireMsg = JSON.stringify({
      topic: 'session',
      event: 'created',
      ts: '2025-01-01T00:00:00Z',
      run_id: null,
      user_id: null,
      data: {},
    })

    act(() => {
      lastWs!.onmessage!(new MessageEvent('message', { data: wireMsg }))
    })

    expect(badHandler).toHaveBeenCalledTimes(1)
    expect(goodHandler).toHaveBeenCalledTimes(1)
    expect(errorSpy).toHaveBeenCalled()

    errorSpy.mockRestore()
  })
})
