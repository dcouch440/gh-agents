import type { CanvasNodeKind } from '../canvasKinds'

type DocumentNodeData = {
  kind: CanvasNodeKind
  label: string
  documenterName: string
  content: string
  protocolStepId: string | null
  documentId: string | null
}

export type { DocumentNodeData }
