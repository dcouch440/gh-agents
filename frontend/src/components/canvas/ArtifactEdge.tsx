import { memo } from 'react'
import type { EdgeProps } from '@xyflow/react'
import { CONNECTOR } from './constants'
import { PipeEdgePath } from './PipeEdgePath'
import { computeBezierPath } from './edges/bezierPath'

function ArtifactEdgeComponent(props: EdgeProps) {
  const { sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition } = props

  const edgePath = computeBezierPath(sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition)

  return (
    <PipeEdgePath
      edgePath={edgePath}
      selected={false}
      interactionWidth={CONNECTOR.INTERACTION_WIDTH}
      sourceX={sourceX}
      sourceY={sourceY}
      targetX={targetX}
      targetY={targetY}
    />
  )
}

const ArtifactEdge = memo(ArtifactEdgeComponent)

export { ArtifactEdge }
