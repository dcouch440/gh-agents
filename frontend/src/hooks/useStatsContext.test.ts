import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useStatsContext } from './useStatsContext'

describe('useStatsContext', () => {
  it('throws error when used outside StatsProvider', () => {
    expect(() => renderHook(() => useStatsContext())).toThrow(
      'useStatsContext must be used within StatsProvider',
    )
  })
})
