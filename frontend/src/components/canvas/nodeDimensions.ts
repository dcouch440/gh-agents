import { CanvasNodeKind } from './canvasKinds'
import type { Rect } from '@/utils/geometry'
import type { ResizeSizeConstraints } from './ResizableNodeShell/types'

// ============================================================================
// Node Dimension Registry — Unified Lookup for All Canvas Node Kinds
// ============================================================================

type NodeDimensionConfig = {
  readonly defaultWidth: number
  readonly defaultHeight: number
  readonly minWidth: number
  readonly minHeight: number
  readonly maxWidth: number
  readonly maxHeight: number
}

/**
 * Canonical dimension data for every `CanvasNodeKind`.
 * Mirrors the per-node `constants.ts` files but in a single queryable structure.
 */
const NODE_DIMENSIONS: Readonly<Record<CanvasNodeKind, NodeDimensionConfig>> = {
  [CanvasNodeKind.STEP]: {
    defaultWidth: 260,
    defaultHeight: 80,
    minWidth: 260,
    minHeight: 80,
    maxWidth: 260,
    maxHeight: 80,
  },
  [CanvasNodeKind.PROTOCOL]: {
    defaultWidth: 560,
    defaultHeight: 500,
    minWidth: 360,
    minHeight: 300,
    maxWidth: 1800,
    maxHeight: 1600,
  },
  [CanvasNodeKind.AGENT]: {
    defaultWidth: 420,
    defaultHeight: 360,
    minWidth: 360,
    minHeight: 300,
    maxWidth: 1200,
    maxHeight: 1000,
  },
  [CanvasNodeKind.CONTEXT]: {
    defaultWidth: 560,
    defaultHeight: 500,
    minWidth: 360,
    minHeight: 300,
    maxWidth: 1800,
    maxHeight: 1600,
  },
  [CanvasNodeKind.INPUT]: {
    defaultWidth: 560,
    defaultHeight: 500,
    minWidth: 360,
    minHeight: 300,
    maxWidth: 1800,
    maxHeight: 1600,
  },
  [CanvasNodeKind.DOCUMENT]: {
    defaultWidth: 420,
    defaultHeight: 360,
    minWidth: 360,
    minHeight: 300,
    maxWidth: 1800,
    maxHeight: 1600,
  },
  [CanvasNodeKind.NOTES]: {
    defaultWidth: 560,
    defaultHeight: 500,
    minWidth: 300,
    minHeight: 240,
    maxWidth: 1200,
    maxHeight: 1200,
  },
  [CanvasNodeKind.SUB_WORKFLOW]: {
    defaultWidth: 180,
    defaultHeight: 56,
    minWidth: 180,
    minHeight: 56,
    maxWidth: 180,
    maxHeight: 56,
  },
}

/** Look up dimension config for a canvas node kind. */
const getNodeDimensions = (kind: CanvasNodeKind): NodeDimensionConfig =>
  NODE_DIMENSIONS[kind]

/**
 * Convert a ReactFlow-shaped node to a `Rect`, using actual dimensions
 * if present or falling back to the kind's defaults.
 */
const nodeToRect = (node: {
  position: { x: number; y: number }
  width?: number | null
  height?: number | null
  data: { kind: CanvasNodeKind }
}): Rect => {
  const dims = NODE_DIMENSIONS[node.data.kind]
  return {
    x: node.position.x,
    y: node.position.y,
    width: node.width ?? dims.defaultWidth,
    height: node.height ?? dims.defaultHeight,
  }
}

/** Bridge to the existing `ResizeSizeConstraints` type used by `ResizableNodeShell`. */
const toResizeConstraints = (kind: CanvasNodeKind): ResizeSizeConstraints => {
  const dims = NODE_DIMENSIONS[kind]
  return {
    minWidth: dims.minWidth,
    minHeight: dims.minHeight,
    maxWidth: dims.maxWidth,
    maxHeight: dims.maxHeight,
  }
}

export { getNodeDimensions, nodeToRect, toResizeConstraints, NODE_DIMENSIONS }
export type { NodeDimensionConfig }
