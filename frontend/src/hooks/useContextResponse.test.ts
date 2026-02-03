import { renderHook, act } from '@testing-library/react'
import { useContextResponse } from './useContextResponse'

const { mockPost } = vi.hoisted(() => ({ mockPost: vi.fn() }))

vi.mock('@/api', () => ({ api: { post: mockPost } }))

describe('useContextResponse', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('submits context response successfully', async () => {
    mockPost.mockResolvedValue(undefined)
    const { result } = renderHook(() => useContextResponse())

    await act(async () => {
      await result.current.mutate({ agent_id: 'agent-001', response: 'yes' })
    })

    expect(mockPost).toHaveBeenCalledWith('/context-response', { agent_id: 'agent-001', response: 'yes' })
    expect(result.current.error).toBeNull()
    expect(result.current.loading).toBe(false)
  })

  it('sets error and throws on failure', async () => {
    mockPost.mockRejectedValue(new Error('Submit failed'))
    const { result } = renderHook(() => useContextResponse())

    await act(async () => {
      await expect(
        result.current.mutate({ agent_id: 'agent-001', response: 'yes' }),
      ).rejects.toThrow('Submit failed')
    })

    expect(result.current.error).toBe('Submit failed')
    expect(result.current.loading).toBe(false)
  })
})
