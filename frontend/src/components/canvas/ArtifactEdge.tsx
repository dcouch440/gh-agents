import { memo } from 'react'
import { getBezierPath } from '@xyflow/react'
import type { EdgeProps } from '@xyflow/react'
import { PIPE } from './constants'
import { PipeEdgePath } from './PipeEdgePath'

type ArtifactEdgeData = { color: string }

function ArtifactEdgeComponent(props: EdgeProps) {
  const { sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, data } = props
  const color = (data as Partial<ArtifactEdgeData> | undefined)?.color ?? '#7d8590'

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
      color={color}
      selected={false}
      isProtocol={true}
      interactionWidth={PIPE.INTERACTION_WIDTH}
    />
  )
}

const ArtifactEdge = memo(ArtifactEdgeComponent)

export { ArtifactEdge }
