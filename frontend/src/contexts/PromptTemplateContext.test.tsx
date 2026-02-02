import { render, screen, waitFor } from '@testing-library/react'
import { PromptTemplateProvider } from './PromptTemplateContext'
import { usePromptTemplateContext } from '@/hooks/usePromptTemplateContext'
import { mockPromptTemplate } from '@/test/fixtures'

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
  const { templates, loading, error } = usePromptTemplateContext()
  if (loading) return <div>loading</div>
  if (error) return <div>error: {error}</div>
  return (
    <div>
      {templates.map((t) => (
        <div key={t.id} data-testid={`template-${t.id}`}>
          {t.name}
        </div>
      ))}
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('PromptTemplateContext', () => {
  describe('PromptTemplateProvider', () => {
    beforeEach(() => {
      vi.clearAllMocks()
      mockGet.mockResolvedValue([mockPromptTemplate])
    })

    it('fetches prompt templates on mount and renders them', async () => {
      render(
        <PromptTemplateProvider>
          <TestConsumer />
        </PromptTemplateProvider>,
      )

      expect(screen.getByText('loading')).toBeInTheDocument()

      await waitFor(() => {
        expect(screen.getByTestId('template-template-001')).toHaveTextContent('Test Template')
      })
    })

    it('handles fetch error', async () => {
      mockGet.mockRejectedValue(new Error('Network error'))

      render(
        <PromptTemplateProvider>
          <TestConsumer />
        </PromptTemplateProvider>,
      )

      await waitFor(() => {
        expect(screen.getByText('error: Network error')).toBeInTheDocument()
      })
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('usePromptTemplateContext must be used within PromptTemplateProvider')
      spy.mockRestore()
    })
  })
})
