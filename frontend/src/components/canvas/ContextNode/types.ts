import type { CanvasNodeKind } from '../canvasKinds'

type ContextNodeData = {
  kind: CanvasNodeKind
  label: string
  content: string
  protocolColor: string | null
  protocolStepId: string | null
}

export type { ContextNodeData }
