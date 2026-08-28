// ============================================================================
// useCanvasSync — Live canvas sync via WebSocket
// ============================================================================
//
// Debounces canvas changes (position, text) and sends them to the backend
// via WebSocket. Create/delete operations are sent immediately.
// Cancels pending debounces when the backend broadcasts BoardElementsUpdated
// (agent wrote files that changed the board).
//
// Every outbound mutation carries a sequence number, and the server acks each
// one once it has been applied. `flushAndWait` uses those acks to answer
// "is the server's copy of the board caught up with mine?" — which the Generate
// button must know, because it triggers work that reads persisted state.

import { useCallback, useEffect, useRef } from 'react'
import { CANVAS_SYNC_POSITION_DEBOUNCE_MS, CANVAS_SYNC_TEXT_DEBOUNCE_MS, WS_TOPIC, WORKFLOW_EVENT } from '@/constants'
import { WS_MSG } from '@/types/ws'
import type {
  CanvasElementMovedMsg,
  CanvasTextChangedMsg,
  CanvasNodeCreatedMsg,
  CanvasEdgeCreatedMsg,
  CanvasNodeDeletedMsg,
  CanvasEdgeDeletedMsg,
  WsWireMessage,
} from '@/types/ws'
import type { ArrowElement, BoardElements, BoxElement } from '../elements'
import { ElementDebouncerMap } from './elementDebouncer'
import { useWebSocket } from '@/hooks/useWebSocket'

type CanvasChange =
  | { readonly kind: 'moved'; readonly elementId: string; readonly x: number; readonly y: number; readonly width: number; readonly height: number }
  | { readonly kind: 'text_changed'; readonly elementId: string; readonly text: string; readonly width: number; readonly height: number }
  | { readonly kind: 'node_created'; readonly box: BoxElement }
  | { readonly kind: 'edge_created'; readonly arrow: ArrowElement }
  | { readonly kind: 'elements_deleted'; readonly deletedIds: ReadonlySet<string>; readonly elements: BoardElements }

type CanvasChangeCallback = (change: CanvasChange) => void

type CanvasSync = {
  readonly handleCanvasChange: CanvasChangeCallback
  /**
   * Flush every pending debounce and resolve once the server has acked all
   * outstanding mutations — or once `timeoutMs` elapses, so a dropped ack can
   * never wedge the caller.
   */
  readonly flushAndWait: (timeoutMs?: number) => Promise<void>
}

/** How long Generate will wait for the board to be durable before proceeding anyway. */
const DEFAULT_FLUSH_TIMEOUT_MS = 3000

const useCanvasSync = (workflowId: string): CanvasSync => {
  // Destructured: `useWebSocket` spreads a fresh object every render, so an
  // effect keyed on the whole context would tear down and re-establish its
  // subscription on each one — which for `subscribe` also means re-sending
  // SUBSCRIBE/UNSUBSCRIBE frames. The individual callbacks are stable.
  const { send, subscribe, subscribeCanvasAck } = useWebSocket()

  // Per-element debouncers stored in refs (not state — no re-renders)
  const positionDebouncerRef = useRef<ElementDebouncerMap<CanvasElementMovedMsg> | null>(null)
  const textDebouncerRef = useRef<ElementDebouncerMap<CanvasTextChangedMsg> | null>(null)

  // The debouncers are built once and outlive any particular `send` identity,
  // so they reach it through a ref rather than capturing it.
  const sendRef = useRef(send)
  useEffect(() => { sendRef.current = send }, [send])

  // Mutations sent but not yet acked, and the waiters blocked on them draining.
  const seqRef = useRef(0)
  const unackedRef = useRef(new Set<number>())
  const waitersRef = useRef(new Set<() => void>())

  const nextSeq = useCallback((): number => {
    seqRef.current += 1
    const seq = seqRef.current
    unackedRef.current.add(seq)
    return seq
  }, [])

  const settleWaiters = useCallback(() => {
    if (unackedRef.current.size > 0) return
    const waiters = Array.from(waitersRef.current)
    waitersRef.current.clear()
    for (const resolve of waiters) resolve()
  }, [])

  // Lazily initialize debouncers with stable send function.
  //
  // Built here rather than in an effect so the very first change on mount has
  // somewhere to go, and never nulled — a StrictMode cleanup runs without a
  // re-render, so a nulled ref would silently swallow every sync until the next
  // render happened to rebuild it.
  positionDebouncerRef.current ??= new ElementDebouncerMap<CanvasElementMovedMsg>(
    CANVAS_SYNC_POSITION_DEBOUNCE_MS,
    (_elementId, payload) => { sendRef.current(payload) },
  )
  textDebouncerRef.current ??= new ElementDebouncerMap<CanvasTextChangedMsg>(
    CANVAS_SYNC_TEXT_DEBOUNCE_MS,
    (_elementId, payload) => { sendRef.current(payload) },
  )

  // Clear acked mutations, and cancel debounces when the agent updates the board
  useEffect(() => {
    const unsubscribe = subscribe(WS_TOPIC.WORKFLOW, (msg: WsWireMessage) => {
      if (msg.event === WORKFLOW_EVENT.BOARD_ELEMENTS_UPDATED) {
        positionDebouncerRef.current?.flushAll()
        textDebouncerRef.current?.flushAll()
      }
    })
    return unsubscribe
  }, [subscribe])

  // Acks arrive on the control channel, not as topic events.
  useEffect(() => {
    const unsubscribe = subscribeCanvasAck((ack) => {
      // The server applies a connection's mutations in order, so an ack for
      // `seq` clears everything at or below it — including any ack we missed.
      for (const pending of Array.from(unackedRef.current)) {
        if (pending <= ack.seq) unackedRef.current.delete(pending)
      }
      if (ack.error !== null) {
        console.warn('[canvas] mutation rejected:', ack.element_id, ack.error)
      }
      settleWaiters()
    })
    return unsubscribe
  }, [subscribeCanvasAck, settleWaiters])

  // Flush on unmount to prevent data loss
  useEffect(() => {
    return () => {
      positionDebouncerRef.current?.flushAll()
      textDebouncerRef.current?.flushAll()
    }
  }, [])

  // Switching workflows: push the outgoing board's pending edits now rather
  // than leaving them on a timer, then drop ack state so a waiter can never
  // block on a mutation belonging to a workflow we have left.
  useEffect(() => {
    const unacked = unackedRef.current
    const waiters = waitersRef.current
    return () => {
      positionDebouncerRef.current?.flushAll()
      textDebouncerRef.current?.flushAll()
      unacked.clear()
      for (const resolve of Array.from(waiters)) resolve()
      waiters.clear()
    }
  }, [workflowId])

  const flushAndWait = useCallback(
    (timeoutMs: number = DEFAULT_FLUSH_TIMEOUT_MS): Promise<void> => {
      positionDebouncerRef.current?.flushAll()
      textDebouncerRef.current?.flushAll()

      if (unackedRef.current.size === 0) return Promise.resolve()

      return new Promise<void>((resolve) => {
        const done = (): void => {
          clearTimeout(timer)
          waitersRef.current.delete(done)
          resolve()
        }
        const timer = setTimeout(() => {
          waitersRef.current.delete(done)
          console.warn('[canvas] timed out waiting for sync acks; proceeding')
          resolve()
        }, timeoutMs)
        waitersRef.current.add(done)
      })
    },
    [],
  )

  const handleCanvasChange: CanvasChangeCallback = useCallback((change: CanvasChange) => {
    switch (change.kind) {
      case 'moved': {
        const msg: CanvasElementMovedMsg = {
          type: WS_MSG.CANVAS_ELEMENT_MOVED,
          workflow_id: workflowId,
          element_id: change.elementId,
          x: change.x,
          y: change.y,
          width: change.width,
          height: change.height,
          seq: nextSeq(),
        }
        positionDebouncerRef.current?.schedule(change.elementId, msg)
        break
      }
      case 'text_changed': {
        const msg: CanvasTextChangedMsg = {
          type: WS_MSG.CANVAS_TEXT_CHANGED,
          workflow_id: workflowId,
          element_id: change.elementId,
          text: change.text,
          seq: nextSeq(),
        }
        textDebouncerRef.current?.schedule(change.elementId, msg)
        break
      }
      case 'node_created': {
        const msg: CanvasNodeCreatedMsg = {
          type: WS_MSG.CANVAS_NODE_CREATED,
          workflow_id: workflowId,
          element_id: change.box.id,
          x: change.box.x,
          y: change.box.y,
          width: change.box.width,
          height: change.box.height,
          text: change.box.text,
          seq: nextSeq(),
        }
        send(msg)
        break
      }
      case 'edge_created': {
        const msg: CanvasEdgeCreatedMsg = {
          type: WS_MSG.CANVAS_EDGE_CREATED,
          workflow_id: workflowId,
          element_id: change.arrow.id,
          source_element_id: change.arrow.sourceBoxId,
          target_element_id: change.arrow.targetBoxId,
          seq: nextSeq(),
        }
        send(msg)
        break
      }
      case 'elements_deleted': {
        for (const id of change.deletedIds) {
          if (change.elements.boxes.has(id)) {
            const msg: CanvasNodeDeletedMsg = {
              type: WS_MSG.CANVAS_NODE_DELETED,
              workflow_id: workflowId,
              element_id: id,
              seq: nextSeq(),
            }
            send(msg)
          } else if (change.elements.arrows.has(id)) {
            const msg: CanvasEdgeDeletedMsg = {
              type: WS_MSG.CANVAS_EDGE_DELETED,
              workflow_id: workflowId,
              element_id: id,
              seq: nextSeq(),
            }
            send(msg)
          }
          // Pens are UI-only — no backend sync needed
        }
        break
      }
    }
  }, [workflowId, send, nextSeq])

  return { handleCanvasChange, flushAndWait }
}

export { useCanvasSync }
export type { CanvasChange, CanvasChangeCallback, CanvasSync }
