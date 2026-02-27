import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useBoardSubmit } from './useBoardSubmit'
import { boardStore } from '@/stores'
import { boardElementStore } from '@/stores/boardElementStore'
import { INITIAL_STATE } from '@/stores/boardStore/_store'
import { addBox, createBox, emptyBoard } from '../elements'

// ── Setup ────────────────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks()
  boardStore.store.setState(INITIAL_STATE)
  boardElementStore.replaceElements(emptyBoard())
})

// ── Tests ────────────────────────────────────────────────────────────────

describe('useBoardSubmit', () => {
  it('handleSubmit serializes elements and calls boardStore.submitBoard', () => {
    const submitSpy = vi.spyOn(boardStore, 'submitBoard').mockResolvedValue()

    const box = createBox(100, 200, 'hello')
    const board = addBox(emptyBoard(), box)
    boardElementStore.replaceElements(board)

    const { result } = renderHook(() => useBoardSubmit('wf-42'))

    act(() => { result.current.handleSubmit() })

    expect(submitSpy).toHaveBeenCalledOnce()
    const [workflowId, elements] = submitSpy.mock.calls[0]!
    expect(workflowId).toBe('wf-42')
    // Should have 2 elements: 1 rectangle + 1 text
    expect(elements).toHaveLength(2)
    expect((elements[0] as Record<string, unknown>)['type']).toBe('rectangle')
    expect((elements[1] as Record<string, unknown>)['type']).toBe('text')
  })

  it('handleSubmit is a no-op when already submitting', () => {
    const submitSpy = vi.spyOn(boardStore, 'submitBoard').mockResolvedValue()
    boardStore.store.setState({ status: 'submitting' })

    const { result } = renderHook(() => useBoardSubmit('wf-1'))

    act(() => { result.current.handleSubmit() })

    expect(submitSpy).not.toHaveBeenCalled()
  })

  it('submits empty array for empty board', () => {
    const submitSpy = vi.spyOn(boardStore, 'submitBoard').mockResolvedValue()

    const { result } = renderHook(() => useBoardSubmit('wf-1'))

    act(() => { result.current.handleSubmit() })

    expect(submitSpy).toHaveBeenCalledWith('wf-1', [])
  })

  it('exposes isSubmitting, error, and status from boardStore', () => {
    boardStore.store.setState({ status: 'error', error: 'Network failure' })

    const { result } = renderHook(() => useBoardSubmit('wf-1'))

    expect(result.current.isSubmitting).toBe(false)
    expect(result.current.error).toBe('Network failure')
    expect(result.current.status).toBe('error')
  })

  it('exposes isSubmitting as true during submit', () => {
    boardStore.store.setState({ status: 'submitting' })

    const { result } = renderHook(() => useBoardSubmit('wf-1'))

    expect(result.current.isSubmitting).toBe(true)
    expect(result.current.status).toBe('submitting')
  })
})
