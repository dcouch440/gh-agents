import { memo } from 'react'
import { getBezierPath } from '@xyflow/react'
import type { EdgeProps } from '@xyflow/react'
import { PIPE } from './constants'
import { PipeEdgePath } from './PipeEdgePath'
import { AGENT_NODE } from './AgentNode'

function AgentEdgeComponent(props: EdgeProps) {
  const { sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition } = props

  const [edgePath] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
  })

  return (
    <PipeEdgePath
      edgePath={edgePath}
      color={AGENT_NODE.ACCENT_COLOR}
      selected={false}
      isProtocol={true}
      interactionWidth={PIPE.INTERACTION_WIDTH}
    />
  )
}

const AgentEdge = memo(AgentEdgeComponent)

export { AgentEdge }
