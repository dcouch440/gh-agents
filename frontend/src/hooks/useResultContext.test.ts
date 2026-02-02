import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useResultContext } from './useResultContext'

describe('useResultContext', () => {
  it('throws error when used outside ResultProvider', () => {
    expect(() => renderHook(() => useResultContext())).toThrow(
      'useResultContext must be used within ResultProvider',
    )
  })
})
