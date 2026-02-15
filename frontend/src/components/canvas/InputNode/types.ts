import type { CanvasNodeKind } from '../canvasKinds'

type InputNodeData = {
  kind: CanvasNodeKind
  label: string
  content: string
  protocolColor: string | null
  protocolStepId: string | null
}

export type { InputNodeData }
