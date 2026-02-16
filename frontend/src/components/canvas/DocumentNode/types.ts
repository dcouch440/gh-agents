import type { CanvasNodeKind } from '../canvasKinds'

type DocumentNodeData = {
  kind: CanvasNodeKind
  label: string
  parentStepName: string
  content: string
  protocolStepId: string | null
  documentId: string | null
}

export type { DocumentNodeData }
