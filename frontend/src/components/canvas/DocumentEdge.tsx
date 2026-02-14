import { memo } from 'react'
import { getBezierPath } from '@xyflow/react'
import type { EdgeProps } from '@xyflow/react'
import { PIPE, PROTOCOL_TYPE_COLORS } from './constants'
import { PipeEdgePath } from './PipeEdgePath'

function DocumentEdgeComponent(props: EdgeProps) {
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
      color={PROTOCOL_TYPE_COLORS['documenter']}
      selected={false}
      isProtocol={true}
      animationDirection="reverse"
      interactionWidth={PIPE.INTERACTION_WIDTH}
    />
  )
}

const DocumentEdge = memo(DocumentEdgeComponent)

export { DocumentEdge }
