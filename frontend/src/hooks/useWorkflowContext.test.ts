import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useWorkflowContext } from './useWorkflowContext'

describe('useWorkflowContext', () => {
  it('throws error when used outside WorkflowProvider', () => {
    expect(() => renderHook(() => useWorkflowContext())).toThrow(
      'useWorkflowContext must be used within WorkflowProvider',
    )
  })
})
