import { renderHook, waitFor } from '@testing-library/react'
import { useDocuments, useDocument } from './useDocuments'
import { mockDocument } from '@/test/fixtures'

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('@/api', () => ({ api: { get: mockGet } }))
vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})

describe('useDocuments', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useDocuments', () => {
    it('fetches and returns documents', async () => {
      mockGet.mockResolvedValue([mockDocument])
      const { result } = renderHook(() => useDocuments())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.documents).toEqual([mockDocument])
    })

    it('sets error on failure', async () => {
      mockGet.mockRejectedValue(new Error('Failed'))
      const { result } = renderHook(() => useDocuments())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.error).toBe('Failed')
    })
  })

  describe('useDocument', () => {
    it('fetches a single document by id', async () => {
      mockGet.mockResolvedValue(mockDocument)
      const { result } = renderHook(() => useDocument('doc-001'))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.document).toEqual(mockDocument)
    })

    it('returns null when id is null', async () => {
      const { result } = renderHook(() => useDocument(null))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.document).toBeNull()
      expect(mockGet).not.toHaveBeenCalled()
    })
  })
})
