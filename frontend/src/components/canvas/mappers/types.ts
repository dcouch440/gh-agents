import type { CanvasNodeKind } from '../canvasKinds'

type ProtocolStepInfo = {
  protocol_type: string
  name: string
  portNames: string[]
}

type ProtocolGroupEntry = {
  protocolColor: string
  protocolStepId: string
}

type StepNodeData = {
  kind: CanvasNodeKind
  label: string
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
  protocolStepId: string | null
  isProtocol: boolean
}

type DocumentDefInfo = {
  id: string
  name: string
  document_id: string | null
  agent_roster_entry_id: string | null
}

type RosterAgentInfo = {
  id: string
  name: string
  child_step_id: string | null
  role_description: string
  depends_on: string[]
}

type StepNodeLookups = {
  agents: ReadonlyMap<string, { name: string; model_id: string }>
  outputSchemas: ReadonlyMap<string, { name: string }>
  stepNames: ReadonlyMap<string, string>
  edges: ReadonlyArray<{ from_step_id: string; to_step_id: string }>
  toolsByAgent: ReadonlyMap<string, string[]>
  protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>
  documentDefsByStep: Readonly<Record<string, ReadonlyArray<DocumentDefInfo>>>
  rosterByStep: Readonly<Record<string, ReadonlyArray<RosterAgentInfo>>>
  notesByStep: Readonly<Record<string, string>>
  documentContentByDefId: Readonly<Record<string, string>>
  protocolGroups: ReadonlyMap<string, ProtocolGroupEntry>
}

type StepEdgeData = {
  sourceColor: string
  isProtocolEdge: boolean
}

export type {
  ProtocolStepInfo,
  ProtocolGroupEntry,
  StepNodeData,
  DocumentDefInfo,
  RosterAgentInfo,
  StepNodeLookups,
  StepEdgeData,
}
