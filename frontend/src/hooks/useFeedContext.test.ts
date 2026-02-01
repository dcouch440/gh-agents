import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useFeedContext } from './useFeedContext'

describe('useFeedContext', () => {
  it('throws error when used outside FeedProvider', () => {
    expect(() => renderHook(() => useFeedContext())).toThrow(
      'useFeedContext must be used within FeedProvider',
    )
  })
})
