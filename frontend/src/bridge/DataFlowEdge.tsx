import type { EdgeProps } from '@xyflow/react'
import { BaseEdge, getSmoothStepPath } from '@xyflow/react'
import type { StepEdge } from './types'

function DataFlowEdge(props: EdgeProps<StepEdge>) {
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
        stroke: selected ? '#1976d2' : data?.hovered ? '#666' : '#999',
        strokeWidth: selected ? 3 : 2,
      }}
    />
  )
}

export { DataFlowEdge }
