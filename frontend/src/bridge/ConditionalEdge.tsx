import type { EdgeProps } from '@xyflow/react'
import { BaseEdge, EdgeLabelRenderer, getSmoothStepPath } from '@xyflow/react'
import type { StepEdge } from './types'

function ConditionalEdge(props: EdgeProps<StepEdge>) {
  const { id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, data, selected } = props

  const [edgePath, labelX, labelY] = getSmoothStepPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  })

  return (
    <>
      <BaseEdge
        id={id}
        path={edgePath}
        style={{
          stroke: selected ? '#ed6c02' : data?.hovered ? '#f57c00' : '#fb8c00',
          strokeWidth: selected ? 3 : 2,
          strokeDasharray: '5 5',
        }}
      />
      {data?.condition && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: 'absolute',
              transform: `translate(-50%, -50%) translate(${String(labelX)}px, ${String(labelY)}px)`,
              pointerEvents: 'all',
              fontSize: 11,
              fontWeight: 500,
              background: '#fff',
              border: '1px solid #fb8c00',
              borderRadius: 4,
              padding: '2px 6px',
              color: '#e65100',
            }}
          >
            {data.condition}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  )
}

export { ConditionalEdge }
