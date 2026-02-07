import { render, screen, waitFor } from '@testing-library/react'
import { act } from 'react'
import { WebSocketProvider } from './WebSocketContext'
import { useWebSocket } from '@/hooks/useWebSocket'
import { AuthProvider } from './AuthContext'
import type { User } from '@/types/user'
import type { WsTopic, WsWireMessage } from '@/types/ws'

// ── Mocks ────────────────────────────────────────────────────────────────────

const mockUser: User = {
  id: 'user-001',
  email: 'test@example.com',
  github_login: null,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('@/api', () => ({
  api: { get: mockGet },
}))

vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return {
    ...actual,
    LS_AUTH_TOKEN: 'test_auth_token',
    WS_URL: 'ws://localhost:3000/ws',
    WS_RECONNECT_BASE_MS: 100,
    WS_RECONNECT_MAX_MS: 500,
  }
})

// ── WebSocket mock ───────────────────────────────────────────────────────────

class MockWebSocket {
  public readyState = WebSocket.CONNECTING
  public url: string
  private listeners = new Map<string, Set<(event: unknown) => void>>()

  constructor(url: string) {
    this.url = url
    ;(global as { mockWsInstance?: MockWebSocket }).mockWsInstance = this
  }

  addEventListener(event: string, handler: (event: unknown) => void) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set())
    }
    this.listeners.get(event)!.add(handler)
  }

  removeEventListener(event: string, handler: (event: unknown) => void) {
    this.listeners.get(event)?.delete(handler)
  }

  send(data: string) {
    if (!(global as { mockWsSentMessages?: string[] }).mockWsSentMessages) {
      ;(global as { mockWsSentMessages?: string[] }).mockWsSentMessages = []
    }
    ;(global as { mockWsSentMessages: string[] }).mockWsSentMessages.push(data)
  }

  close() {
    this.readyState = WebSocket.CLOSED
    this.emit('close', {})
  }

  // Test helpers
  emit(event: string, data: unknown) {
    const handlers = this.listeners.get(event)
    if (handlers) {
      for (const handler of handlers) {
        handler(data)
      }
    }
  }

  open() {
    this.readyState = WebSocket.OPEN
    this.emit('open', {})
  }

  message(data: unknown) {
    this.emit('message', { data: JSON.stringify(data) })
  }

  error() {
    this.emit('error', {})
  }
}

// @ts-expect-error - mocking WebSocket
// eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
global.WebSocket = MockWebSocket

// ── Test consumer ────────────────────────────────────────────────────────────

function TestConsumer() {
  const { status, latency, subscribe } = useWebSocket()

  return (
    <div>
      <div data-testid="status">{status}</div>
      <div data-testid="latency">{latency === null ? 'null' : String(latency)}</div>
      <button
        onClick={() => {
          subscribe('workflow' as WsTopic, (msg: WsWireMessage) => {
            const el = document.createElement('div')
            el.setAttribute('data-testid', 'message')
            el.textContent = `${msg.topic}:${msg.event}`
            document.body.appendChild(el)
          })
        }}
      >
        subscribe
      </button>
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('WebSocketContext', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    ;(global as { mockWsInstance?: MockWebSocket }).mockWsInstance = undefined
    ;(global as { mockWsSentMessages?: string[] }).mockWsSentMessages = []

    localStorage.setItem('test_auth_token', 'test-token-123')
    mockGet.mockResolvedValue(mockUser)
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe('WebSocketProvider', () => {
    it('initializes as disconnected without token', async () => {
      localStorage.clear()

      render(
        <AuthProvider>
          <WebSocketProvider>
            <TestConsumer />
          </WebSocketProvider>
        </AuthProvider>,
      )

      await waitFor(() => {
        expect(screen.queryByText('loading')).not.toBeInTheDocument()
      })

      expect(screen.getByTestId('status')).toHaveTextContent('disconnected')
    })

    it('connects when token is available', async () => {
      render(
        <AuthProvider>
          <WebSocketProvider>
            <TestConsumer />
          </WebSocketProvider>
        </AuthProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('status')).toHaveTextContent('connecting')
      })

      const ws = (global as { mockWsInstance?: MockWebSocket }).mockWsInstance
      expect(ws).toBeDefined()
      expect(ws?.url).toContain('ws://localhost:3000/ws?token=test-token-123')

      act(() => {
        ws?.open()
      })

      await waitFor(() => {
        expect(screen.getByTestId('status')).toHaveTextContent('connected')
      })
    })

    it('subscribes to topics and receives wire messages', async () => {
      render(
        <AuthProvider>
          <WebSocketProvider>
            <TestConsumer />
          </WebSocketProvider>
        </AuthProvider>,
      )

      const ws = (global as { mockWsInstance?: MockWebSocket }).mockWsInstance!
      await waitFor(() => expect(ws).toBeDefined())

      act(() => {
        ws.open()
      })

      await waitFor(() => {
        expect(screen.getByTestId('status')).toHaveTextContent('connected')
      })

      act(() => {
        screen.getByText('subscribe').click()
      })

      // Check subscription message uses topic-based format
      const sentMessages = (global as { mockWsSentMessages: string[] }).mockWsSentMessages
      expect(sentMessages.some((msg) => {
        const parsed = JSON.parse(msg) as { type: string; topics?: string[] }
        return parsed.type === 'subscribe' && parsed.topics?.includes('workflow')
      })).toBe(true)

      // Send a wire message
      act(() => {
        ws.message({
          topic: 'workflow',
          event: 'step_started',
          ts: '2024-01-01T00:00:00Z',
          run_id: 'run-123',
          user_id: null,
          data: { workflow_id: 'wf-1', step_id: 'step-1', step_name: 'Research' },
        })
      })

      await waitFor(() => {
        const messageEl = screen.getByTestId('message')
        expect(messageEl).toHaveTextContent('workflow:step_started')
      })
    })

    it('handles control messages without dispatching to topic handlers', async () => {
      render(
        <AuthProvider>
          <WebSocketProvider>
            <TestConsumer />
          </WebSocketProvider>
        </AuthProvider>,
      )

      const ws = (global as { mockWsInstance?: MockWebSocket }).mockWsInstance!
      await waitFor(() => expect(ws).toBeDefined())

      act(() => {
        ws.open()
      })

      await waitFor(() => {
        expect(screen.getByTestId('status')).toHaveTextContent('connected')
      })

      // Subscribe to workflow
      act(() => {
        screen.getByText('subscribe').click()
      })

      // Send a control message (pong)
      act(() => {
        ws.message({
          type: 'pong',
          client_ts: new Date(Date.now() - 50).toISOString(),
          server_ts: new Date().toISOString(),
        })
      })

      // Should not create a topic message element
      expect(screen.queryByTestId('message')).not.toBeInTheDocument()
    })

    it('reconnects with exponential backoff on disconnect', async () => {
      vi.useFakeTimers()

      render(
        <AuthProvider>
          <WebSocketProvider>
            <TestConsumer />
          </WebSocketProvider>
        </AuthProvider>,
      )

      await vi.waitFor(() => {
        const ws1 = (global as { mockWsInstance?: MockWebSocket }).mockWsInstance
        expect(ws1).toBeDefined()
      })

      const ws1 = (global as { mockWsInstance?: MockWebSocket }).mockWsInstance!

      act(() => {
        ws1.open()
      })

      expect(screen.getByTestId('status')).toHaveTextContent('connected')

      act(() => {
        ws1.close()
      })

      expect(screen.getByTestId('status')).toHaveTextContent('disconnected')

      act(() => {
        vi.advanceTimersByTime(100)
      })

      const ws2 = (global as { mockWsInstance?: MockWebSocket }).mockWsInstance!
      expect(ws2).toBeDefined()
      expect(ws2).not.toBe(ws1)

      act(() => {
        ws2.open()
      })

      expect(screen.getByTestId('status')).toHaveTextContent('connected')

      vi.useRealTimers()
    })

    it('syncs subscriptions on reconnect', async () => {
      vi.useFakeTimers()

      render(
        <AuthProvider>
          <WebSocketProvider>
            <TestConsumer />
          </WebSocketProvider>
        </AuthProvider>,
      )

      await vi.waitFor(() => {
        const ws1 = (global as { mockWsInstance?: MockWebSocket }).mockWsInstance
        expect(ws1).toBeDefined()
      })

      const ws1 = (global as { mockWsInstance?: MockWebSocket }).mockWsInstance!

      act(() => {
        ws1.open()
      })

      expect(screen.getByTestId('status')).toHaveTextContent('connected')

      // Subscribe to a topic
      act(() => {
        screen.getByText('subscribe').click()
      })

      const sentMessages1 = [...(global as { mockWsSentMessages: string[] }).mockWsSentMessages]
      expect(sentMessages1.some((msg) => {
        const parsed = JSON.parse(msg) as { topics?: string[] }
        return parsed.topics?.includes('workflow')
      })).toBe(true)

      // Clear sent messages
      ;(global as { mockWsSentMessages: string[] }).mockWsSentMessages = []

      // Close and reconnect
      act(() => {
        ws1.close()
      })

      act(() => {
        vi.advanceTimersByTime(100)
      })

      const ws2 = (global as { mockWsInstance?: MockWebSocket }).mockWsInstance!

      act(() => {
        ws2.open()
      })

      expect(screen.getByTestId('status')).toHaveTextContent('connected')

      // Check that subscription was re-sent with topics
      const sentMessages2 = (global as { mockWsSentMessages: string[] }).mockWsSentMessages
      expect(sentMessages2.some((msg) => {
        const parsed = JSON.parse(msg) as { topics?: string[] }
        return parsed.topics?.includes('workflow')
      })).toBe(true)

      vi.useRealTimers()
    })

    it('throws when useWebSocket is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useWebSocket must be used within WebSocketProvider')
      spy.mockRestore()
    })
  })
})
