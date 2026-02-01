import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useRoutingContext } from './useRoutingContext'

describe('useRoutingContext', () => {
  it('throws error when used outside RoutingProvider', () => {
    expect(() => renderHook(() => useRoutingContext())).toThrow(
      'useRoutingContext must be used within RoutingProvider',
    )
  })
})
