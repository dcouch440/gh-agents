import type { Rect, Side } from '@/utils/geometry'
import type { CanvasNodeKind } from '../canvasKinds'
import type { PortPlacement } from '../portPlacements'

// ============================================================================
// Layout Types — Shared Vocabulary for Canvas Layout Algorithms
// ============================================================================

/** A node represented as its essential layout data. */
type LayoutNode = {
  readonly id: string
  readonly kind: CanvasNodeKind
  readonly rect: Rect
}

/** An edge between two layout nodes, optionally with port info. */
type LayoutEdge = {
  readonly id: string
  readonly sourceId: string
  readonly targetId: string
  readonly sourcePort: PortPlacement | null
  readonly targetPort: PortPlacement | null
}

/** Axis for alignment guide lines. */
type AlignmentAxis = 'horizontal' | 'vertical'

/** A single alignment guide line emitted by a node. */
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

/** A detected overlap between a moved node and another node. */
type Overlap = {
  readonly nodeId: string
  readonly overlapRect: Rect
  readonly pushDirection: Side
  readonly pushDistance: number
}

export type {
  LayoutNode,
  LayoutEdge,
  AlignmentAxis,
  AlignmentGuide,
  SnapEdge,
  SnapCandidate,
  SnapResult,
  Overlap,
}
