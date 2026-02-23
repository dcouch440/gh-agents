import type { Rect, Side } from '@/utils/geometry'

// ============================================================================
// Spatial Types — Shared Vocabulary for 2D Layout Algorithms
// ============================================================================

/** A rectangle identified by a string ID. Minimal shape for layout algorithms. */
type LayoutRect = {
  readonly id: string
  readonly rect: Rect
}

/** Axis for alignment guide lines. */
type AlignmentAxis = 'horizontal' | 'vertical'

/** A single alignment guide line emitted by a rectangle. */
type AlignmentGuide = {
  readonly axis: AlignmentAxis
  /** The coordinate of the guide line (x for vertical, y for horizontal). */
  readonly position: number
  readonly anchorNodeId: string
}

/** Which edge of the drag rect matched the guide. */
type SnapEdge = 'start' | 'end' | 'center'

/** A guide that a dragged rect is near enough to snap to. */
type SnapCandidate = {
  readonly guide: AlignmentGuide
  readonly distance: number
  readonly snapEdge: SnapEdge
}

/** The result of computing a snap for a dragged rect. */
type SnapResult = {
  readonly snappedX: number
  readonly snappedY: number
  readonly activeGuides: readonly AlignmentGuide[]
}

/** A detected overlap between a moved rect and another rect. */
type Overlap = {
  readonly nodeId: string
  readonly overlapRect: Rect
  readonly pushDirection: Side
  readonly pushDistance: number
}

/** A rectangle with padding applied for gap enforcement in collision queries. */
type OccupiedRect = {
  readonly id: string
  readonly rect: Rect
  /** The rect expanded by padding on all sides — used for collision testing. */
  readonly paddedRect: Rect
}

export type {
  LayoutRect,
  AlignmentAxis,
  AlignmentGuide,
  SnapEdge,
  SnapCandidate,
  SnapResult,
  Overlap,
  OccupiedRect,
}
