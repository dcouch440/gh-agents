import { memo } from 'react'
import type { EdgeProps } from '@xyflow/react'
import { PIPE, GREYSCALE_ACCENT } from './constants'
import { PipeEdgePath } from './PipeEdgePath'
import { computeBezierPath } from './edges/bezierPath'

type ArtifactEdgeData = {
  color: string
}

function ArtifactEdgeComponent(props: EdgeProps) {
  const { sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, data } = props
  const rawData = data as Partial<ArtifactEdgeData> | undefined
  const color = rawData?.color ?? GREYSCALE_ACCENT

  const edgePath = computeBezierPath(sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition)

  return (
    <PipeEdgePath
      edgePath={edgePath}
      color={color}
      selected={false}
      isProtocol={true}
      interactionWidth={PIPE.INTERACTION_WIDTH}
    />
  )
}

const ArtifactEdge = memo(ArtifactEdgeComponent)

export { ArtifactEdge }
