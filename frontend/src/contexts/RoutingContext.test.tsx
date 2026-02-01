import { render, screen, waitFor } from '@testing-library/react'
import { RoutingProvider } from './RoutingContext'
import { useRoutingContext } from '../hooks/useRoutingContext'
import { mockRoutingEvent, mockRoutingEventCompleted } from '../test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

let wsHandler: ((data: unknown) => void) | null = null

vi.mock('../hooks/useWebSocket', () => ({
  useWebSocket: () => ({
    status: 'connected' as const,
    subscribe: (_channel: string, handler: (data: unknown) => void) => {
      wsHandler = handler
      return () => { wsHandler = null }
    },
  }),
}))

// ── Test consumer ────────────────────────────────────────────────────────────

function TestConsumer() {
  const { events } = useRoutingContext()
  return (
    <div>
      <div data-testid="count">{events.length}</div>
      {events.map((e) => (
        <div key={e.id} data-testid={`event-${e.id}`}>
          {e.tool_name}:{e.status}
        </div>
      ))}
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('RoutingContext', () => {
  describe('RoutingProvider', () => {
    beforeEach(() => {
      wsHandler = null
      vi.clearAllMocks()
    })

    it('starts with empty events', () => {
      render(
        <RoutingProvider>
          <TestConsumer />
        </RoutingProvider>,
      )

      expect(screen.getByTestId('count')).toHaveTextContent('0')
    })

    it('appends routing events via WS', async () => {
      render(
        <RoutingProvider>
          <TestConsumer />
        </RoutingProvider>,
      )

      wsHandler?.({ event: mockRoutingEvent })

      await waitFor(() => {
        expect(screen.getByTestId('event-route-001')).toHaveTextContent('search_files:pending')
      })
    })

    it('updates completed events in place', async () => {
      render(
        <RoutingProvider>
          <TestConsumer />
        </RoutingProvider>,
      )

      wsHandler?.({ event: mockRoutingEvent })
      await waitFor(() => {
        expect(screen.getByTestId('event-route-001')).toHaveTextContent('search_files:pending')
      })

      wsHandler?.({ event: mockRoutingEventCompleted, completed: true })
      await waitFor(() => {
        expect(screen.getByTestId('event-route-001')).toHaveTextContent('search_files:completed')
      })

      // Should still be 1 event, not 2
      expect(screen.getByTestId('count')).toHaveTextContent('1')
    })

    it('caps at 200 events', async () => {
      render(
        <RoutingProvider>
          <TestConsumer />
        </RoutingProvider>,
      )

      for (let i = 0; i < 210; i++) {
        wsHandler?.({ event: { ...mockRoutingEvent, id: `route-${i}` } })
      }

      await waitFor(() => {
        expect(screen.getByTestId('count')).toHaveTextContent('200')
      })
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useRoutingContext must be used within RoutingProvider')
      spy.mockRestore()
    })
  })
})
