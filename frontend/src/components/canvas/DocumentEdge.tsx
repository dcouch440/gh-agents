import { memo } from 'react'
import { BaseEdge, getBezierPath } from '@xyflow/react'
import type { EdgeProps } from '@xyflow/react'
import { CANVAS, PROTOCOL_TYPE_COLORS } from './constants'
import './edgeFlow.css'

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
    <BaseEdge
      path={edgePath}
      style={{
        stroke: PROTOCOL_TYPE_COLORS['documenter'],
        strokeWidth: CANVAS.EDGE_STROKE_WIDTH,
        strokeDasharray: CANVAS.EDGE_DASH_PATTERN,
        opacity: CANVAS.EDGE_OPACITY_PROTOCOL,
        animation: `edgeFlow ${CANVAS.EDGE_FLOW_DURATION} linear infinite reverse`,
      }}
    />
  )
}

const DocumentEdge = memo(DocumentEdgeComponent)

export { DocumentEdge }
