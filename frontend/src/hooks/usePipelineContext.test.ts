import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { usePipelineContext } from './usePipelineContext'

describe('usePipelineContext', () => {
  it('throws error when used outside PipelineProvider', () => {
    expect(() => renderHook(() => usePipelineContext())).toThrow(
      'usePipelineContext must be used within PipelineProvider',
    )
  })
})
