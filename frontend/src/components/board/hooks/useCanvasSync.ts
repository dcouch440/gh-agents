// ============================================================================
// useCanvasSync — Live canvas sync via WebSocket
// ============================================================================
//
// Debounces canvas changes (position, text) and sends them to the backend
// via WebSocket. Create/delete operations are sent immediately.
// Cancels pending debounces when the backend broadcasts BoardElementsUpdated
// (agent wrote files that changed the board).

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

const useCanvasSync = (workflowId: string): CanvasChangeCallback => {
  const ws = useWebSocket()

  // Per-element debouncers stored in refs (not state — no re-renders)
  const positionDebouncerRef = useRef<ElementDebouncerMap<CanvasElementMovedMsg> | null>(null)
  const textDebouncerRef = useRef<ElementDebouncerMap<CanvasTextChangedMsg> | null>(null)

  // Lazily initialize debouncers with stable send function
  positionDebouncerRef.current ??= new ElementDebouncerMap<CanvasElementMovedMsg>(
    CANVAS_SYNC_POSITION_DEBOUNCE_MS,
    (_elementId, payload) => { ws.send(payload) },
  )
  textDebouncerRef.current ??= new ElementDebouncerMap<CanvasTextChangedMsg>(
    CANVAS_SYNC_TEXT_DEBOUNCE_MS,
    (_elementId, payload) => { ws.send(payload) },
  )

  // Cancel all debounces when agent updates the board
  useEffect(() => {
    const unsubscribe = ws.subscribe(WS_TOPIC.WORKFLOW, (msg: WsWireMessage) => {
      if (msg.event === WORKFLOW_EVENT.BOARD_ELEMENTS_UPDATED) {
        positionDebouncerRef.current?.flushAll()
        textDebouncerRef.current?.flushAll()
      }
    })
    return unsubscribe
  }, [ws])

  // Flush on unmount to prevent data loss
  useEffect(() => {
    return () => {
      positionDebouncerRef.current?.flushAll()
      textDebouncerRef.current?.flushAll()
    }
  }, [])

  // Dispose when workflow changes
  useEffect(() => {
    return () => {
      positionDebouncerRef.current?.dispose()
      textDebouncerRef.current?.dispose()
      positionDebouncerRef.current = null
      textDebouncerRef.current = null
    }
  }, [workflowId])

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
        }
        ws.send(msg)
        break
      }
      case 'edge_created': {
        const msg: CanvasEdgeCreatedMsg = {
          type: WS_MSG.CANVAS_EDGE_CREATED,
          workflow_id: workflowId,
          element_id: change.arrow.id,
          source_element_id: change.arrow.sourceBoxId,
          target_element_id: change.arrow.targetBoxId,
        }
        ws.send(msg)
        break
      }
      case 'elements_deleted': {
        for (const id of change.deletedIds) {
          if (change.elements.boxes.has(id)) {
            const msg: CanvasNodeDeletedMsg = {
              type: WS_MSG.CANVAS_NODE_DELETED,
              workflow_id: workflowId,
              element_id: id,
            }
            ws.send(msg)
          } else if (change.elements.arrows.has(id)) {
            const msg: CanvasEdgeDeletedMsg = {
              type: WS_MSG.CANVAS_EDGE_DELETED,
              workflow_id: workflowId,
              element_id: id,
            }
            ws.send(msg)
          }
          // Pens are UI-only — no backend sync needed
        }
        break
      }
    }
  }, [workflowId, ws])

  return handleCanvasChange
}

export { useCanvasSync }
export type { CanvasChange, CanvasChangeCallback }
