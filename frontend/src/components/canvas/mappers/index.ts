export { toRFNodes } from './nodes'
export { toRFEdges, toAgentEdges, toNotesEdges } from './edges'
export { nodeDataEqual } from './equality'
export { computeProtocolGroups, isWorkforceStep } from './protocolGroups'

export type {
  StepNodeData,
  StepNodeLookups,
  StepEdgeData,
  ProtocolGroupEntry,
  ProtocolStepInfo,
} from './types'

// Re-export ContextNodeData for backwards compatibility
export type { ContextNodeData } from '../ContextNode'
