// ============================================================================
// bridge/transforms — Pure WorkflowStep ↔ React Flow Node transforms
// ============================================================================

import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import type { StepExecutionState } from '@/stores/workflowExecutionStore'
import type { StepNode, StepEdge } from './types'

// ── Constants ────────────────────────────────────────────────────────────────

const STEP_TYPE_TO_NODE_TYPE: Record<string, StepNode['type']> = {
  single: 'singleStep',
  for_each: 'forEachStep',
  room: 'roomStep',
}

const DEFAULT_NODE_TYPE: StepNode['type'] = 'singleStep'

// ── WorkflowStep → React Flow Node ──────────────────────────────────────────

const stepToNode = (
  step: WorkflowStep,
  executionState: StepExecutionState | null,
  selected: boolean,
  hovered: boolean,
): StepNode => ({
  id: step.id,
  type: STEP_TYPE_TO_NODE_TYPE[step.execution_mode] ?? DEFAULT_NODE_TYPE,
  position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
  selected,
  data: {
    stepId: step.id,
    workflowId: step.workflow_id,
    name: step.name ?? step.execution_mode,
    stepType: step.execution_mode,
    agentId: step.agent_id,
    promptTemplateId: step.prompt_template_id,
    outputSchemaId: step.output_schema_id,
    forEachLabelField: step.for_each_label_field,
    executionState,
    hovered,
  },
})

// ── WorkflowStepEdge → React Flow Edge ──────────────────────────────────────

const edgeToFlowEdge = (
  edge: WorkflowStepEdge,
  selected: boolean,
  hovered: boolean,
): StepEdge => ({
  id: edge.id,
  source: edge.from_step_id,
  target: edge.to_step_id,
  type: 'dataFlow',
  selected,
  data: {
    edgeId: edge.id,
    hovered,
  },
})

// ── React Flow Node → Position Update ───────────────────────────────────────

const nodeToPositionUpdate = (node: StepNode): { position_x: number; position_y: number } => ({
  position_x: node.position.x,
  position_y: node.position.y,
})

// ── Export ────────────────────────────────────────────────────────────────────

export { stepToNode, edgeToFlowEdge, nodeToPositionUpdate, STEP_TYPE_TO_NODE_TYPE }
