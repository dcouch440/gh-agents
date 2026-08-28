import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useCanvasSync } from './useCanvasSync'
import { createBox, emptyBoard } from '../elements'
import type { CanvasAckMsg } from '@/types/ws'

// ── Mock the socket ──────────────────────────────────────────────────────

const { mockSend, ackHandlers } = vi.hoisted(() => ({
  mockSend: vi.fn(),
  ackHandlers: new Set<(ack: CanvasAckMsg) => void>(),
}))

vi.mock('@/hooks/useWebSocket', () => ({
  useWebSocket: () => ({
    send: mockSend,
    subscribe: () => () => {},
    subscribeCanvasAck: (handler: (ack: CanvasAckMsg) => void) => {
      ackHandlers.add(handler)
      return () => ackHandlers.delete(handler)
    },
  }),
}))

const ack = (seq: number, error: string | null = null): void => {
  act(() => {
    for (const handler of ackHandlers) {
      handler({ type: 'canvas_ack', seq, element_id: 'e', error })
    }
  })
}

const WF = 'wf-1'

// ── Tests ────────────────────────────────────────────────────────────────

describe('useCanvasSync', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    ackHandlers.clear()
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('stamps an increasing seq on every mutation', () => {
    const { result } = renderHook(() => useCanvasSync(WF))

    act(() => {
      result.current.handleCanvasChange({ kind: 'node_created', box: createBox(0, 0) })
      result.current.handleCanvasChange({ kind: 'node_created', box: createBox(10, 10) })
    })

    expect(mockSend).toHaveBeenCalledTimes(2)
    const first = mockSend.mock.calls[0]?.[0] as { seq: number }
    const second = mockSend.mock.calls[1]?.[0] as { seq: number }
    expect(first.seq).toBe(1)
    expect(second.seq).toBe(2)
  })

  it('resolves flushAndWait immediately when nothing is outstanding', async () => {
    const { result } = renderHook(() => useCanvasSync(WF))

    await expect(result.current.flushAndWait()).resolves.toBeUndefined()
  })

  it('waits for the ack before resolving', async () => {
    const { result } = renderHook(() => useCanvasSync(WF))

    act(() => {
      result.current.handleCanvasChange({ kind: 'node_created', box: createBox(0, 0) })
    })

    let settled = false
    const pending = result.current.flushAndWait().then(() => { settled = true })

    await Promise.resolve()
    expect(settled).toBe(false)

    ack(1)
    await pending
    expect(settled).toBe(true)
  })

  it('treats an ack as covering every earlier seq', async () => {
    const { result } = renderHook(() => useCanvasSync(WF))

    act(() => {
      result.current.handleCanvasChange({ kind: 'node_created', box: createBox(0, 0) })
      result.current.handleCanvasChange({ kind: 'node_created', box: createBox(1, 1) })
      result.current.handleCanvasChange({ kind: 'node_created', box: createBox(2, 2) })
    })

    const pending = result.current.flushAndWait()
    // Only the newest ack arrives — the server applies in order, so it implies
    // the first two landed.
    ack(3)

    await expect(pending).resolves.toBeUndefined()
  })

  it('resolves on timeout when no ack ever arrives', async () => {
    const { result } = renderHook(() => useCanvasSync(WF))

    act(() => {
      result.current.handleCanvasChange({ kind: 'node_created', box: createBox(0, 0) })
    })

    const pending = result.current.flushAndWait(3000)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3000)
    })

    await expect(pending).resolves.toBeUndefined()
  })

  it('stops waiting when a mutation is rejected', async () => {
    const { result } = renderHook(() => useCanvasSync(WF))

    act(() => {
      result.current.handleCanvasChange({ kind: 'node_created', box: createBox(0, 0) })
    })

    const pending = result.current.flushAndWait()
    ack(1, 'Canvas sync queue full')

    await expect(pending).resolves.toBeUndefined()
  })

  it('flushes debounced text through on flushAndWait', async () => {
    const { result } = renderHook(() => useCanvasSync(WF))

    act(() => {
      result.current.handleCanvasChange({
        kind: 'text_changed', elementId: 'box-1', text: 'a description', width: 10, height: 10,
      })
    })
    // Still sitting in the debouncer — this is the window Generate used to lose.
    expect(mockSend).not.toHaveBeenCalled()

    const pending = result.current.flushAndWait()
    expect(mockSend).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'canvas_text_changed', text: 'a description' }),
    )

    ack(1)
    await expect(pending).resolves.toBeUndefined()
  })

  it('sends a delete for each removed element', () => {
    const { result } = renderHook(() => useCanvasSync(WF))
    const box = createBox(0, 0)
    let board = emptyBoard()
    board = { ...board, boxes: new Map([[box.id, box]]) }

    act(() => {
      result.current.handleCanvasChange({
        kind: 'elements_deleted', deletedIds: new Set([box.id]), elements: board,
      })
    })

    expect(mockSend).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'canvas_node_deleted', element_id: box.id }),
    )
  })
})
