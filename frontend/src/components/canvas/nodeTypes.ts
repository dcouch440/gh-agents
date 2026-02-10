import type { NodeTypes } from '@xyflow/react'
import { StepNode } from './StepNode'
import { DocumenterNode } from './DocumenterNode'
import { DocumentNode } from './DocumentNode'

const nodeTypes: NodeTypes = {
  stepNode: StepNode,
  documenterNode: DocumenterNode,
  documentNode: DocumentNode,
}

export { nodeTypes }
