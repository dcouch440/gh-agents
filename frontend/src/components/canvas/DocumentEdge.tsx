import { memo } from 'react'
import { BaseEdge, getBezierPath } from '@xyflow/react'
import type { EdgeProps } from '@xyflow/react'

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
        stroke: '#D4793E',
        strokeWidth: 2,
        strokeDasharray: '6 4',
        opacity: 0.5,
      }}
    />
  )
}

const DocumentEdge = memo(DocumentEdgeComponent)

export { DocumentEdge }
