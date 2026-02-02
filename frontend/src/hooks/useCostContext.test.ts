import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useCostContext } from './useCostContext'

describe('useCostContext', () => {
  it('throws error when used outside CostProvider', () => {
    expect(() => renderHook(() => useCostContext())).toThrow(
      'useCostContext must be used within CostProvider',
    )
  })
})
