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
export { detectOverlaps, resolveOverlaps } from './collisionDetection'
export { computeTowerPositions, computeAllTowerPositions } from './autoLayout'
export type { ProtocolDimensions, NodeDimensions } from './autoLayout'
export { TOWER_LAYOUT } from './autoLayoutConfig'
export { notchToGrid } from './gridNotch'
