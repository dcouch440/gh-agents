import type { CanvasNodeKind } from '../canvasKinds'

type AgentNodeData = {
  kind: CanvasNodeKind
  label: string
  roleDescription: string
  parentStepName: string
  protocolStepId: string | null
  rosterAgentId: string
  capabilities: string[]
}

export type { AgentNodeData }
