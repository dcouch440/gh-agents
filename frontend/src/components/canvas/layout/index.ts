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

export { buildAlignmentGuides, findSnapCandidates, computeSnap } from './snapAlignment'
export { detectOverlaps, resolveOverlaps } from './collisionDetection'
