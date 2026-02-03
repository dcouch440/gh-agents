import { renderHook, waitFor } from '@testing-library/react'
import { useTools } from './useTools'
import { mockTool } from '@/test/fixtures'

const { mockList } = vi.hoisted(() => ({ mockList: vi.fn() }))

vi.mock('@/api', () => ({ api: { tools: { list: mockList } } }))
describe('useTools', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('fetches and returns tools', async () => {
    mockList.mockResolvedValue({ items: [mockTool] })
    const { result } = renderHook(() => useTools())

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.tools).toEqual([mockTool])
    expect(result.current.error).toBeNull()
  })

  it('sets error on failure', async () => {
    mockList.mockRejectedValue(new Error('Failed'))
    const { result } = renderHook(() => useTools())

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.error).toBe('Failed')
  })
})
