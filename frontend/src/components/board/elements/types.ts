// ============================================================================
// Board Element Types — Internal Data Model
// ============================================================================
//
// These types represent the board's internal state. They are deliberately
// different from Excalidraw's format — `serialize.ts` handles the conversion.
// Boxes store their own text directly; arrows reference boxes by ID with
// anchor positions stored as side + ratio.

import type { Point, Side } from '@/utils/geometry'

// ── Elements ───────────────────────────────────────────────────────────────

type BoxElement = {
  readonly id: string
  readonly type: 'box'
  readonly x: number
  readonly y: number
  readonly width: number
  readonly height: number
  readonly text: string
}

type AnchorPoint = {
  readonly side: Side
  readonly ratio: number // 0..1 along the side (0.5 = midpoint)
}

type ArrowElement = {
  readonly id: string
  readonly type: 'arrow'
  readonly sourceBoxId: string
  readonly targetBoxId: string
  readonly sourceAnchor: AnchorPoint
  readonly targetAnchor: AnchorPoint
}

type CanvasElement = BoxElement | ArrowElement

// ── Board State ────────────────────────────────────────────────────────────

type BoardElements = {
  readonly boxes: ReadonlyMap<string, BoxElement>
  readonly arrows: ReadonlyMap<string, ArrowElement>
  readonly boxOrder: readonly string[]
}

// ── Selection ──────────────────────────────────────────────────────────────

type SelectionState = {
  readonly selectedIds: ReadonlySet<string>
  readonly marquee: MarqueeRect | null
}

type MarqueeRect = {
  readonly x: number
  readonly y: number
  readonly width: number
  readonly height: number
}

// ── Interaction Mode ───────────────────────────────────────────────────────

type InteractionMode =
  | { readonly type: 'idle' }
  | { readonly type: 'dragging'; readonly elementId: string; readonly offsetX: number; readonly offsetY: number }
  | { readonly type: 'drawing-arrow'; readonly sourceBoxId: string; readonly sourceAnchor: AnchorPoint; readonly cursorX: number; readonly cursorY: number }
  | { readonly type: 'selecting'; readonly startX: number; readonly startY: number }
  | { readonly type: 'panning'; readonly startX: number; readonly startY: number; readonly startPanX: number; readonly startPanY: number }
  | { readonly type: 'editing'; readonly boxId: string }
  | { readonly type: 'resizing'; readonly boxId: string; readonly handle: ResizeHandle; readonly startX: number; readonly startY: number; readonly startBox: { x: number; y: number; width: number; height: number } }

type ResizeHandle = 'nw' | 'ne' | 'sw' | 'se' | 'n' | 's' | 'e' | 'w'

// ── Active Tool ────────────────────────────────────────────────────────────

type ActiveTool = 'select' | 'box' | 'arrow'

// ── Viewport ───────────────────────────────────────────────────────────────

type ViewportState = {
  readonly panX: number
  readonly panY: number
  readonly zoom: number
}

// ── Drawing Arrow Preview ──────────────────────────────────────────────────

type DrawingArrow = {
  readonly sourceBoxId: string
  readonly sourceAnchor: AnchorPoint
  readonly cursorX: number
  readonly cursorY: number
} | null

// ── Screen-to-canvas coordinate conversion ─────────────────────────────────

const screenToCanvas = (
  screenX: number,
  screenY: number,
  viewport: ViewportState,
  containerRect: DOMRect,
): Point => ({
  x: (screenX - containerRect.left - viewport.panX) / viewport.zoom,
  y: (screenY - containerRect.top - viewport.panY) / viewport.zoom,
})

export { screenToCanvas }
export type {
  BoxElement,
  ArrowElement,
  AnchorPoint,
  CanvasElement,
  BoardElements,
  SelectionState,
  MarqueeRect,
  InteractionMode,
  ResizeHandle,
  ActiveTool,
  ViewportState,
  DrawingArrow,
}
