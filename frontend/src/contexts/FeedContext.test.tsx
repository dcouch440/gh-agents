import { render, screen } from '@testing-library/react'
import { FeedProvider } from './FeedContext'
import { useFeedContext } from '@/hooks/useFeedContext'

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

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useFeedContext must be used within FeedProvider')
      spy.mockRestore()
    })
  })
})
