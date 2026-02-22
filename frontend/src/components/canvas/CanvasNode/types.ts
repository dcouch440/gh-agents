import type { CanvasNodeKind } from '../canvasKinds'

const NodeVariant = {
  WORKFORCE: 'workforce',
  MANAGER: 'manager',
  ROOM: 'room',
  BLANK: 'blank',
  AGENT: 'agent',
  CONTEXT: 'context',
  INPUT: 'input',
  STEP: 'step',
  SUB_WORKFLOW: 'sub_workflow',
} as const

type NodeVariant = (typeof NodeVariant)[keyof typeof NodeVariant]

/** Shared fields present on every union member. */
type NodeDataBase = {
  kind: CanvasNodeKind
  label: string
  protocolStepId: string | null
}

/** workforce, manager, room, blank — tabbed layout with chat, live stream, archetype tab */
type TabbedNodeData = NodeDataBase & {
  variant: 'workforce' | 'manager' | 'room' | 'blank'
  description: string
  documentNames: string[]
  rosterNames: string[]
  roomId: string | null
  upstreamStepNames: string[]
  promptValue: string
  modelId: string | null
  agentName: string | null
}

/** Agent artifact within a workforce — tabbed layout with stream + info */
type AgentNodeData = NodeDataBase & {
  variant: 'agent'
  rosterAgentId: string | null
  roleDescription: string | null
  capabilities: string[]
  parentStepName: string | null
}

/** context, input — resizable editor layout */
type EditorNodeData = NodeDataBase & {
  variant: 'context' | 'input'
  content: string
  protocolColor: string | null
}

/** Legacy fallback for single, for_each, etc. — card layout */
type CardNodeData = NodeDataBase & {
  variant: 'step'
  stepType: string
  agentId: string | null
  promptTemplateId: string | null
  outputSchemaId: string | null
  agentName: string | null
  modelId: string | null
  outputSchemaName: string | null
  upstreamStepNames: string[]
  toolNames: string[]
  protocolType: string | null
  protocolName: string | null
  protocolPortNames: string[]
  protocolColor: string | null
  isProtocol: boolean
}

/** Sub-workflow — compact layout */
type CompactNodeData = NodeDataBase & {
  variant: 'sub_workflow'
  templateName: string | null
}

type CanvasNodeData =
  | TabbedNodeData
  | AgentNodeData
  | EditorNodeData
  | CardNodeData
  | CompactNodeData

type LayoutMode = 'tabbed' | 'editor' | 'card' | 'compact'

export { NodeVariant }
export type {
  CanvasNodeData,
  TabbedNodeData,
  AgentNodeData,
  EditorNodeData,
  CardNodeData,
  CompactNodeData,
  LayoutMode,
}
