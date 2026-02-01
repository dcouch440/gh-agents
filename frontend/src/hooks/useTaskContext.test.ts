import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useTaskContext } from './useTaskContext'

describe('useTaskContext', () => {
  it('throws error when used outside TaskProvider', () => {
    expect(() => renderHook(() => useTaskContext())).toThrow(
      'useTaskContext must be used within TaskProvider',
    )
  })
})
