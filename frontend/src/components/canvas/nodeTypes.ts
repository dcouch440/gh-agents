import type { NodeTypes } from '@xyflow/react'
import { StepNode } from './StepNode'
import { DynamicNode } from './DynamicNode'
import { ContextNode } from './ContextNode'
import { DocumentNode } from './DocumentNode'

const nodeTypes: NodeTypes = {
  stepNode: StepNode,
  dynamicNode: DynamicNode,
  contextNode: ContextNode,
  documentNode: DocumentNode,
}

export { nodeTypes }
