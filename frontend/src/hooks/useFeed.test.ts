import { renderHook, waitFor } from '@testing-library/react'
import { useFeed } from './useFeed'

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
