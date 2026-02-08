import { memo } from 'react'
import { BaseEdge, getBezierPath } from '@xyflow/react'
import type { EdgeProps } from '@xyflow/react'

function StepEdgeComponent(props: EdgeProps) {
  const {
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    selected,
  } = props

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
        stroke: selected ? '#3b82f6' : '#7d8590',
        strokeWidth: 2,
        opacity: selected ? 0.8 : 0.4,
        transition: 'stroke 150ms ease, opacity 150ms ease',
      }}
    />
  )
}

const StepEdge = memo(StepEdgeComponent)

export { StepEdge }
