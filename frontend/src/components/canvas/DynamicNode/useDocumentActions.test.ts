import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useDocumentActions } from './useDocumentActions'

const createDocumentDef = vi.fn<(stepId: string, body: unknown) => void>()
const deleteDocumentDef = vi.fn<(stepId: string, defId: string) => void>()

vi.mock('@/stores', () => ({
  workflowStore: {
    createDocumentDef: (stepId: string, body: unknown): void => createDocumentDef(stepId, body),
    deleteDocumentDef: (stepId: string, defId: string): void => deleteDocumentDef(stepId, defId),
  },
}))

describe('useDocumentActions', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('starts with adding=false', () => {
    const { result } = renderHook(() => useDocumentActions('step-1'))
    expect(result.current.adding).toBe(false)
  })

  it('onAdd sets adding to true', () => {
    const { result } = renderHook(() => useDocumentActions('step-1'))

    act(() => {
      result.current.onAdd()
    })

    expect(result.current.adding).toBe(true)
  })

  it('onCancelAdd sets adding back to false', () => {
    const { result } = renderHook(() => useDocumentActions('step-1'))

    act(() => {
      result.current.onAdd()
    })
    expect(result.current.adding).toBe(true)

    act(() => {
      result.current.onCancelAdd()
    })
    expect(result.current.adding).toBe(false)
  })

  it('onSubmitNew calls createDocumentDef and sets adding to false', () => {
    const { result } = renderHook(() => useDocumentActions('step-1'))

    act(() => {
      result.current.onAdd()
    })

    const body = { name: 'New Doc', description: 'Desc', target_length: 500 }
    act(() => {
      result.current.onSubmitNew(body)
    })

    expect(createDocumentDef).toHaveBeenCalledWith('step-1', body)
    expect(result.current.adding).toBe(false)
  })

  it('onRemove calls deleteDocumentDef with stepId and defId', () => {
    const { result } = renderHook(() => useDocumentActions('step-1'))

    act(() => {
      result.current.onRemove('def-abc')
    })

    expect(deleteDocumentDef).toHaveBeenCalledWith('step-1', 'def-abc')
  })

  it('uses the stepId passed to the hook', () => {
    const { result } = renderHook(() => useDocumentActions('step-999'))

    const body = { name: 'Doc', description: '', target_length: 100 }
    act(() => {
      result.current.onSubmitNew(body)
    })

    expect(createDocumentDef).toHaveBeenCalledWith('step-999', body)
  })
})
