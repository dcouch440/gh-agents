import type { EdgeProps } from '@xyflow/react'
import { BaseEdge, getSmoothStepPath } from '@xyflow/react'
import type { StepEdge } from './types'

function ConditionalEdge(props: EdgeProps<StepEdge>) {
  const { id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, data, selected } = props

  const [edgePath] = getSmoothStepPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  })

  return (
    <BaseEdge
      id={id}
      path={edgePath}
      style={{
        stroke: selected ? '#ed6c02' : data?.hovered ? '#f57c00' : '#fb8c00',
        strokeWidth: selected ? 3 : 2,
        strokeDasharray: '5 5',
      }}
    />
  )
}

export { ConditionalEdge }
