import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useChatContext } from './useChatContext'

describe('useChatContext', () => {
  it('throws error when used outside ChatProvider', () => {
    expect(() => renderHook(() => useChatContext())).toThrow(
      'useChatContext must be used within ChatProvider',
    )
  })
})
