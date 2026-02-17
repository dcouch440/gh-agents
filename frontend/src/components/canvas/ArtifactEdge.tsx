import { memo } from 'react'
import { useReactFlow } from '@xyflow/react'
import type { EdgeProps } from '@xyflow/react'
import { PIPE, GREYSCALE_ACCENT } from './constants'
import { PipeEdgePath } from './PipeEdgePath'
import { computeOrthogonalPath, findObstaclesInPath, computeCorridorPath } from './edges/orthogonalPath'

type ArtifactEdgeData = {
  color: string
  avoidObstacles?: boolean
}

function ArtifactEdgeComponent(props: EdgeProps) {
  const { source, target, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, data } = props
  const rawData = data as Partial<ArtifactEdgeData> | undefined
  const color = rawData?.color ?? GREYSCALE_ACCENT
  const { getNodes } = useReactFlow()

  let edgePath: string
  if (rawData?.avoidObstacles) {
    const nodes = getNodes()
    const excludeIds = new Set([source, target])
    const obstacles = findObstaclesInPath(nodes, sourceX, sourceY, targetX, targetY, excludeIds)
    edgePath = obstacles.length > 0
      ? computeCorridorPath(sourceX, sourceY, targetX, targetY, obstacles)
      : computeOrthogonalPath(sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition)
  } else {
    edgePath = computeOrthogonalPath(sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition)
  }

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
