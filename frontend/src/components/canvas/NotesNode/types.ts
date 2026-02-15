import type { CanvasNodeKind } from '../canvasKinds'

type NotesNodeData = {
  kind: CanvasNodeKind
  label: string
  stepName: string
  content: string
  protocolStepId: string | null
}

export type { NotesNodeData }
