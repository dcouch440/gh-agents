// ── Types ────────────────────────────────────────────────────────────────
export type {
  LayoutRect,
  AlignmentAxis,
  AlignmentGuide,
  SnapEdge,
  SnapCandidate,
  SnapResult,
  Overlap,
  OccupiedRect,
} from './types'

// ── Geometry Primitives ──────────────────────────────────────────────────
export { lerpPoint } from './lerpPoint'
export { midpoint } from './midpoint'
export { snapToGridMin } from './snapToGridMin'
export { assignParallelTracks } from './assignParallelTracks'

// ── Snap Alignment ───────────────────────────────────────────────────────
export { buildAlignmentGuides } from './buildAlignmentGuides'
export { findSnapCandidates } from './findSnapCandidates'
export { computeSnap } from './computeSnap'
export { computeMagneticSnap } from './computeMagneticSnap'

// ── Collision Detection ──────────────────────────────────────────────────
export { detectOverlaps } from './detectOverlaps'
export { resolveOverlaps } from './resolveOverlaps'

// ── Occupancy Index ──────────────────────────────────────────────────────
export { buildOccupancyIndex } from './buildOccupancyIndex'
export { isOccupied } from './isOccupied'
export { addToOccupancy } from './addToOccupancy'
export { updateOccupancy } from './updateOccupancy'
export { occupancyBounds } from './occupancyBounds'

// ── Free Space ───────────────────────────────────────────────────────────
export { findFreeSpace } from './findFreeSpace'
export type { FindFreeSpaceConfig } from './findFreeSpace'
