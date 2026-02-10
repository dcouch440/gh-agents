import { Collections } from '@/utils/collections'

const HighlightMode = {
  NONE: 'none',
  HOVER: 'hover',
  SELECT: 'select',
} as const

type HighlightMode = (typeof HighlightMode)[keyof typeof HighlightMode]

const CanvasNodeKind = {
  CONTEXT: 'context',
  DOCUMENT: 'document',
  PROTOCOL: 'protocol',
  STEP: 'step',
} as const

type CanvasNodeKind = (typeof CanvasNodeKind)[keyof typeof CanvasNodeKind]

const HOVER_ELIGIBLE_KINDS = Collections.toSet<CanvasNodeKind>(['context', 'document'])

export { HighlightMode, CanvasNodeKind, HOVER_ELIGIBLE_KINDS }
export type { HighlightMode, CanvasNodeKind }
