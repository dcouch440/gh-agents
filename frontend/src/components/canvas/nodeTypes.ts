import type { NodeTypes } from '@xyflow/react'
import { StepNode } from './StepNode'
import { DocumenterNode } from './DocumenterNode'
import { ContextNode } from './ContextNode'
import { DocumentNode } from './DocumentNode'

const nodeTypes: NodeTypes = {
  stepNode: StepNode,
  documenterNode: DocumenterNode,
  contextNode: ContextNode,
  documentNode: DocumentNode,
}

export { nodeTypes }
