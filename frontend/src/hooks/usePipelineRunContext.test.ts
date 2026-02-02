import { describe, it, expect, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import { usePipelineRunContext } from './usePipelineRunContext'

vi.mock('@/hooks/useWebSocket', () => ({
  useWebSocket: () => ({
    status: 'connected' as const,
    subscribe: vi.fn(() => vi.fn()),
    subscribeRun: vi.fn(() => vi.fn()),
    unsubscribeRun: vi.fn(),
    send: vi.fn(),
  }),
}))

describe('usePipelineRunContext', () => {
  it('throws when used outside PipelineRunProvider', () => {
    expect(() => renderHook(() => usePipelineRunContext())).toThrow(
      'usePipelineRunContext must be used within PipelineRunProvider',
    )
  })
})
