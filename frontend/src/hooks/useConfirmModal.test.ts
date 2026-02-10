import {describe, it, expect, vi} from 'vitest'
import {renderHook, act} from '@testing-library/react'
import {useConfirmModal} from './useConfirmModal'

describe('useConfirmModal', () => {
  it('initializes with closed state', () => {
    const {result} = renderHook(() => useConfirmModal())

    expect(result.current.open).toBe(false)
    expect(result.current.loading).toBe(false)
    expect(result.current.error).toBeNull()
  })

  it('opens with correct options', () => {
    const {result} = renderHook(() => useConfirmModal())

    act(() => {
      result.current.openConfirm({
        title: 'Delete Item',
        message: 'Are you sure?',
        confirmText: 'Delete',
        confirmColor: 'error',
        onConfirm: vi.fn().mockResolvedValue(undefined),
      })
    })

    expect(result.current.open).toBe(true)
    expect(result.current.title).toBe('Delete Item')
    expect(result.current.message).toBe('Are you sure?')
    expect(result.current.confirmText).toBe('Delete')
    expect(result.current.confirmColor).toBe('error')
  })

  it('uses default values for optional fields', () => {
    const {result} = renderHook(() => useConfirmModal())

    act(() => {
      result.current.openConfirm({
        title: 'Confirm',
        message: 'Proceed?',
        onConfirm: vi.fn().mockResolvedValue(undefined),
      })
    })

    expect(result.current.confirmText).toBe('Confirm')
    expect(result.current.cancelText).toBe('Cancel')
    expect(result.current.confirmColor).toBe('primary')
  })

  it('closes and resets state', () => {
    const {result} = renderHook(() => useConfirmModal())

    act(() => {
      result.current.openConfirm({
        title: 'Test',
        message: 'Test message',
        onConfirm: vi.fn().mockResolvedValue(undefined),
      })
    })

    expect(result.current.open).toBe(true)

    act(() => {
      result.current.closeConfirm()
    })

    expect(result.current.open).toBe(false)
    expect(result.current.loading).toBe(false)
    expect(result.current.error).toBeNull()
  })

  it('handles async confirmation success', async () => {
    const {result} = renderHook(() => useConfirmModal())
    const asyncAction = vi.fn().mockResolvedValue(undefined)

    act(() => {
      result.current.openConfirm({
        title: 'Delete',
        message: 'Delete this?',
        onConfirm: asyncAction,
      })
    })

    await act(async () => {
      await result.current.handleConfirm()
    })

    expect(asyncAction).toHaveBeenCalledTimes(1)
    expect(result.current.open).toBe(false)
    expect(result.current.loading).toBe(false)
  })

  it('handles async confirmation failure', async () => {
    const {result} = renderHook(() => useConfirmModal())
    const error = new Error('Operation failed')
    const asyncAction = vi.fn().mockRejectedValue(error)

    act(() => {
      result.current.openConfirm({
        title: 'Delete',
        message: 'Delete this?',
        onConfirm: asyncAction,
      })
    })

    await act(async () => {
      await result.current.handleConfirm()
    })

    expect(asyncAction).toHaveBeenCalledTimes(1)
    expect(result.current.open).toBe(true) // Should stay open on error
    expect(result.current.loading).toBe(false)
    expect(result.current.error).toBe('Operation failed')
  })

  it('sets loading during async operation', async () => {
    const {result} = renderHook(() => useConfirmModal())
    const asyncAction = vi.fn().mockResolvedValue(undefined)

    act(() => {
      result.current.openConfirm({
        title: 'Delete',
        message: 'Delete this?',
        onConfirm: asyncAction,
      })
    })

    await act(async () => {
      await result.current.handleConfirm()
    })

    expect(result.current.loading).toBe(false)
    expect(asyncAction).toHaveBeenCalledTimes(1)
  })

  it('handles non-Error rejection', async () => {
    const {result} = renderHook(() => useConfirmModal())
    const asyncAction = vi.fn().mockRejectedValue('String error')

    act(() => {
      result.current.openConfirm({
        title: 'Delete',
        message: 'Delete this?',
        onConfirm: asyncAction,
      })
    })

    await act(async () => {
      await result.current.handleConfirm()
    })

    expect(result.current.error).toBe('Operation failed')
  })

  it('does nothing if handleConfirm called without pending action', async () => {
    const {result} = renderHook(() => useConfirmModal())

    await act(async () => {
      await result.current.handleConfirm()
    })

    expect(result.current.open).toBe(false)
    expect(result.current.loading).toBe(false)
  })
})
