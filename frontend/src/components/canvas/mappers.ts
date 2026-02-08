import type { Node, Edge } from '@xyflow/react'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'

type StepNodeData = {
  label: string
  stepType: string
  agentId: string | null
  promptTemplateId: string | null
  outputSchemaId: string | null
}

const toRFNodes = (steps: WorkflowStep[]): Node<StepNodeData>[] =>
  steps.map((step) => ({
    id: step.id,
    type: 'stepNode',
    position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
    data: {
      label: step.name ?? step.execution_mode,
      stepType: step.execution_mode,
      agentId: step.agent_id,
      promptTemplateId: step.prompt_template_id,
      outputSchemaId: step.output_schema_id,
    },
  }))

const toRFEdges = (edges: WorkflowStepEdge[]): Edge[] =>
  edges.map((edge) => ({
    id: edge.id,
    type: 'stepEdge',
    source: edge.from_step_id,
    target: edge.to_step_id,
  }))

export { toRFNodes, toRFEdges }
export type { StepNodeData }
