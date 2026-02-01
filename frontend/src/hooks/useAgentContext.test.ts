import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useAgentContext } from './useAgentContext'

describe('useAgentContext', () => {
  it('throws error when used outside AgentProvider', () => {
    expect(() => renderHook(() => useAgentContext())).toThrow(
      'useAgentContext must be used within AgentProvider',
    )
  })
})
