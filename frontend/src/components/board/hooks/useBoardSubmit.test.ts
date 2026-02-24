import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import type { ExcalidrawImperativeAPI } from '@excalidraw/excalidraw/types'
import { useBoardSubmit } from './useBoardSubmit'
import { boardStore } from '@/stores'
import { INITIAL_STATE } from '@/stores/boardStore/_store'

// ── Excalidraw API Mock ──────────────────────────────────────────────────

const mockGetSceneElements = vi.fn(() => [{ id: 'el-1', type: 'rectangle' }])

const mockExcalidrawApi = {
  getSceneElements: mockGetSceneElements,
} as unknown as ExcalidrawImperativeAPI

// ── Setup ────────────────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks()
  boardStore.store.setState(INITIAL_STATE)
})

// ── Tests ────────────────────────────────────────────────────────────────

describe('useBoardSubmit', () => {
  it('handleSubmit is a no-op when API is not set', () => {
    const submitSpy = vi.spyOn(boardStore, 'submitBoard').mockResolvedValue()
    const { result } = renderHook(() => useBoardSubmit('wf-1'))

    act(() => { result.current.handleSubmit() })

    expect(submitSpy).not.toHaveBeenCalled()
  })

  it('handleSubmit is a no-op when already submitting', () => {
    const submitSpy = vi.spyOn(boardStore, 'submitBoard').mockResolvedValue()
    boardStore.store.setState({ status: 'submitting' })

    const { result } = renderHook(() => useBoardSubmit('wf-1'))

    act(() => { result.current.setExcalidrawApi(mockExcalidrawApi) })
    act(() => { result.current.handleSubmit() })

    expect(submitSpy).not.toHaveBeenCalled()
  })

  it('reads elements and calls boardStore.submitBoard', () => {
    const submitSpy = vi.spyOn(boardStore, 'submitBoard').mockResolvedValue()

    const { result } = renderHook(() => useBoardSubmit('wf-42'))

    act(() => { result.current.setExcalidrawApi(mockExcalidrawApi) })
    act(() => { result.current.handleSubmit() })

    expect(mockGetSceneElements).toHaveBeenCalledOnce()
    expect(submitSpy).toHaveBeenCalledWith('wf-42', [{ id: 'el-1', type: 'rectangle' }])
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
