// ============================================================================
// bridge/nodeTypes — Custom React Flow node component registry
// ============================================================================

import { SingleStepNode } from './SingleStepNode'
import { ForEachStepNode } from './ForEachStepNode'
import { RoomStepNode } from './RoomStepNode'

const nodeTypes = {
  singleStep: SingleStepNode,
  forEachStep: ForEachStepNode,
  roomStep: RoomStepNode,
}

export { nodeTypes }
