import type { CanvasNodeKind } from '../canvasKinds'

type SubWorkflowNodeData = {
  kind: CanvasNodeKind
  label: string
  templateName: string | null
}

export type { SubWorkflowNodeData }
