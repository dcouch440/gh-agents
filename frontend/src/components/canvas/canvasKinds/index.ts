import { Collections } from '@/utils/collections'

const HighlightMode = {
  NONE: 'none',
  HOVER: 'hover',
  SELECT: 'select',
} as const

type HighlightMode = (typeof HighlightMode)[keyof typeof HighlightMode]

const CanvasNodeKind = {
  AGENT: 'agent',
  CONTEXT: 'context',
  DOCUMENT: 'document',
  INPUT: 'input',
  PROTOCOL: 'protocol',
  STEP: 'step',
  SUB_WORKFLOW: 'sub_workflow',
} as const

type CanvasNodeKind = (typeof CanvasNodeKind)[keyof typeof CanvasNodeKind]

const HOVER_ELIGIBLE_KINDS = Collections.toSet<CanvasNodeKind>(['agent', 'context', 'document', 'input'])

export { HighlightMode, CanvasNodeKind, HOVER_ELIGIBLE_KINDS }
export type { HighlightMode, CanvasNodeKind }
