import { renderHook, waitFor } from '@testing-library/react'
import { useFeed } from './useFeed'

vi.mock('../constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('../constants')
  return { ...actual, USE_MOCK_DATA: false }
})

describe('useFeed', () => {
  it('returns empty items when not using mock data', async () => {
    const { result } = renderHook(() => useFeed())

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.items).toEqual([])
    expect(result.current.error).toBeNull()
  })
})
