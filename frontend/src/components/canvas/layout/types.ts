import type { CanvasNodeKind } from '../canvasKinds'
import type { PortPlacement } from '../portPlacements'
import type { LayoutRect } from '@/utils/spatial'

// Re-export spatial types for canvas consumers
export type {
  AlignmentAxis,
  AlignmentGuide,
  SnapEdge,
  SnapCandidate,
  SnapResult,
  Overlap,
} from '@/utils/spatial'

// ============================================================================
// Canvas-Specific Layout Types
// ============================================================================

/** A canvas node with kind, extending the generic LayoutRect. */
type LayoutNode = LayoutRect & {
  readonly kind: CanvasNodeKind
}

/** An edge between two layout nodes, optionally with port info. */
type LayoutEdge = {
  readonly id: string
  readonly sourceId: string
  readonly targetId: string
  readonly sourcePort: PortPlacement | null
  readonly targetPort: PortPlacement | null
}

export type { LayoutNode, LayoutEdge }
