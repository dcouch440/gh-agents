import type { NodeTypes } from '@xyflow/react'
import { StepNode } from './StepNode'
import { DynamicNode } from './DynamicNode'
import { ContextNode } from './ContextNode'
import { InputNode } from './InputNode'
import { SubWorkflowNode } from './SubWorkflowNode'

const nodeTypes: NodeTypes = {
  stepNode: StepNode,
  dynamicNode: DynamicNode,
  contextNode: ContextNode,
  inputNode: InputNode,
  subWorkflowNode: SubWorkflowNode,
}

export { nodeTypes }
