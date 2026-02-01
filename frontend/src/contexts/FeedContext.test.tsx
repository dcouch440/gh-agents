import { render, screen, waitFor } from '@testing-library/react'
import { FeedProvider } from './FeedContext'
import { useFeedContext } from '../hooks/useFeedContext'
import { mockFeedItem } from '../test/fixtures'

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
  const { items } = useFeedContext()
  return (
    <div>
      <div data-testid="count">{items.length}</div>
      {items.map((item) => (
        <div key={item.id} data-testid={`feed-${item.id}`}>
          {item.content}
        </div>
      ))}
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('FeedContext', () => {
  describe('FeedProvider', () => {
    beforeEach(() => {
      wsHandler = null
      vi.clearAllMocks()
    })

    it('starts with empty items', () => {
      render(
        <FeedProvider>
          <TestConsumer />
        </FeedProvider>,
      )

      expect(screen.getByTestId('count')).toHaveTextContent('0')
    })

    it('appends feed items via WS', async () => {
      render(
        <FeedProvider>
          <TestConsumer />
        </FeedProvider>,
      )

      wsHandler?.({ item: mockFeedItem })

      await waitFor(() => {
        expect(screen.getByTestId('feed-feed-001')).toHaveTextContent('Agent started working')
      })
    })

    it('prepends new items (newest first)', async () => {
      render(
        <FeedProvider>
          <TestConsumer />
        </FeedProvider>,
      )

      wsHandler?.({ item: mockFeedItem })
      wsHandler?.({ item: { ...mockFeedItem, id: 'feed-002', content: 'Second event' } })

      await waitFor(() => {
        expect(screen.getByTestId('count')).toHaveTextContent('2')
      })

      const items = screen.getAllByTestId(/^feed-/)
      expect(items[0]).toHaveTextContent('Second event')
      expect(items[1]).toHaveTextContent('Agent started working')
    })

    it('caps at 200 items', async () => {
      render(
        <FeedProvider>
          <TestConsumer />
        </FeedProvider>,
      )

      for (let i = 0; i < 210; i++) {
        wsHandler?.({ item: { ...mockFeedItem, id: `feed-${i}` } })
      }

      await waitFor(() => {
        expect(screen.getByTestId('count')).toHaveTextContent('200')
      })
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useFeedContext must be used within FeedProvider')
      spy.mockRestore()
    })
  })
})
