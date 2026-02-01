import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useWebSocket } from './useWebSocket'

describe('useWebSocket', () => {
  it('throws error when used outside WebSocketProvider', () => {
    expect(() => renderHook(() => useWebSocket())).toThrow(
      'useWebSocket must be used within WebSocketProvider',
    )
  })
})
