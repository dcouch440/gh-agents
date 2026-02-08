// ============================================================================
// bridge/types — React Flow ↔ Store bridge types
// ============================================================================

import type { Node, Edge } from '@xyflow/react'
import type { StepExecutionState } from '@/stores/workflowExecutionStore'

// ── Node Data ────────────────────────────────────────────────────────────────

type StepNodeData = {
  stepId: string
  workflowId: string
  name: string
  stepType: string
  agentId: string | null
  promptTemplateId: string | null
  outputSchemaId: string | null
  forEachLabelField: string | null
  executionState: StepExecutionState | null
  hovered: boolean
}

type StepNode = Node<StepNodeData, 'singleStep' | 'forEachStep' | 'roomStep'>

// ── Edge Data ────────────────────────────────────────────────────────────────

type EdgeData = {
  edgeId: string
  hovered: boolean
}

type StepEdge = Edge<EdgeData>

// ── Export ────────────────────────────────────────────────────────────────────

export type { StepNodeData, StepNode, EdgeData, StepEdge }
