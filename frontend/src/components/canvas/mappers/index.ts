export { toRFNodes } from './nodes'
export { toRFEdges, toDocumentEdges } from './edges'
export { nodeDataEqual } from './equality'
export { computeProtocolGroups, isDocumenterStep } from './protocolGroups'

export type {
  StepNodeData,
  StepNodeLookups,
  StepEdgeData,
  ProtocolGroupEntry,
  ProtocolStepInfo,
} from './types'

// Re-export ContextNodeData for backwards compatibility
export type { ContextNodeData } from '../ContextNode'
