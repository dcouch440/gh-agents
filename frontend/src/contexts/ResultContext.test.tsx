import { render, screen, waitFor } from '@testing-library/react'
import { ResultProvider } from './ResultContext'
import { useResultContext } from '@/hooks/useResultContext'
import { mockResult } from '@/test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('@/api', () => ({
  api: { get: mockGet },
}))

vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})

// ── Test consumer ────────────────────────────────────────────────────────────

function TestConsumer() {
  const { results, loading, error } = useResultContext()
  if (loading) return <div>loading</div>
  if (error) return <div>error: {error}</div>
  return (
    <div>
      <div data-testid="result-count">{results.length}</div>
      {results.map((r) => (
        <div key={r.id} data-testid={`result-${r.id}`}>
          {r.output}
        </div>
      ))}
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('ResultContext', () => {
  describe('ResultProvider', () => {
    beforeEach(() => {
      vi.clearAllMocks()
      mockGet.mockResolvedValue([mockResult])
    })

    it('fetches results on mount', async () => {
      render(
        <ResultProvider>
          <TestConsumer />
        </ResultProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('result-count')).toHaveTextContent('1')
      })

      expect(screen.getByTestId(`result-${mockResult.id}`)).toHaveTextContent('Task completed successfully')
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useResultContext must be used within ResultProvider')
      spy.mockRestore()
    })
  })
})
