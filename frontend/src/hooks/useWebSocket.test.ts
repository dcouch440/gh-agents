import { renderHook } from '@testing-library/react'
import { useWebSocket } from './useWebSocket'

vi.mock('@/api', () => ({ api: {} }))

describe('useWebSocket', () => {
  it('throws when used outside WebSocketProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    expect(() => renderHook(() => useWebSocket())).toThrow(
      'useWebSocket must be used within WebSocketProvider',
    )
    spy.mockRestore()
  })
})
