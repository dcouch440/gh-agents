import type { Node, Edge } from '@xyflow/react'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'

type StepNodeData = {
  label: string
  stepType: string
  description: string | null
  agentId: string | null
  promptTemplateId: string | null
  outputSchemaId: string | null
}

const toRFNodes = (
  steps: WorkflowStep[],
  selectedIds: ReadonlySet<string>,
): Node<StepNodeData>[] =>
  steps.map((step) => ({
    id: step.id,
    type: 'stepNode',
    position: { x: step.position_x, y: step.position_y },
    selected: selectedIds.has(step.id),
    data: {
      label: step.name,
      stepType: step.step_type,
      description: step.description,
      agentId: step.agent_id,
      promptTemplateId: step.prompt_template_id,
      outputSchemaId: step.output_schema_id,
    },
  }))

const toRFEdges = (
  edges: WorkflowStepEdge[],
  selectedIds: ReadonlySet<string>,
): Edge[] =>
  edges.map((edge) => ({
    id: edge.id,
    type: 'stepEdge',
    source: edge.from_step_id,
    target: edge.to_step_id,
    selected: selectedIds.has(edge.id),
    data: { condition: edge.condition },
  }))

export { toRFNodes, toRFEdges }
export type { StepNodeData }
