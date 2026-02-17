export type {
  LayoutNode,
  LayoutEdge,
  AlignmentAxis,
  AlignmentGuide,
  SnapEdge,
  SnapCandidate,
  SnapResult,
  Overlap,
} from './types'

export { buildAlignmentGuides, findSnapCandidates, computeSnap, computeMagneticSnap } from './snapAlignment'
export { detectOverlaps, resolveOverlaps, resolveOverlapsConstrained } from './collisionDetection'
export type { NodeTopologyRole } from './collisionDetection'
export { computeAutoLayout } from './autoLayout'
export { AUTO_LAYOUT } from './autoLayoutConfig'
export { notchToGrid } from './gridNotch'
