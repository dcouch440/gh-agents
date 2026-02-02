import { render, screen, waitFor } from '@testing-library/react'
import { OutputSchemaProvider } from './OutputSchemaContext'
import { useOutputSchemaContext } from '@/hooks/useOutputSchemaContext'
import { mockOutputSchema } from '@/test/fixtures'

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
  const { schemas, loading, error } = useOutputSchemaContext()
  if (loading) return <div>loading</div>
  if (error) return <div>error: {error}</div>
  return (
    <div>
      {schemas.map((s) => (
        <div key={s.id} data-testid={`schema-${s.id}`}>
          {s.name}
        </div>
      ))}
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('OutputSchemaContext', () => {
  describe('OutputSchemaProvider', () => {
    beforeEach(() => {
      vi.clearAllMocks()
      mockGet.mockResolvedValue([mockOutputSchema])
    })

    it('fetches schemas on mount and renders them', async () => {
      render(
        <OutputSchemaProvider>
          <TestConsumer />
        </OutputSchemaProvider>,
      )

      expect(screen.getByText('loading')).toBeInTheDocument()

      await waitFor(() => {
        expect(screen.getByTestId('schema-schema-001')).toHaveTextContent('Test Schema')
      })
    })

    it('handles fetch error', async () => {
      mockGet.mockRejectedValue(new Error('Network error'))

      render(
        <OutputSchemaProvider>
          <TestConsumer />
        </OutputSchemaProvider>,
      )

      await waitFor(() => {
        expect(screen.getByText('error: Network error')).toBeInTheDocument()
      })
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useOutputSchemaContext must be used within OutputSchemaProvider')
      spy.mockRestore()
    })
  })
})
