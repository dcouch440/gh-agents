import { memo } from 'react'
import { BaseEdge, EdgeLabelRenderer, getBezierPath, useReactFlow } from '@xyflow/react'
import type { EdgeProps } from '@xyflow/react'
import CloseIcon from '@mui/icons-material/Close'
import IconButton from '@mui/material/IconButton'

function StepEdgeComponent(props: EdgeProps) {
  const {
    id,
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    selected,
  } = props

  const { deleteElements } = useReactFlow()

  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
  })

  const handleDelete = (event: React.MouseEvent) => {
    event.stopPropagation()
    event.preventDefault()
    void deleteElements({ edges: [{ id }] })
  }

  return (
    <>
      <BaseEdge
        path={edgePath}
        style={{
          stroke: selected ? '#3b82f6' : '#7d8590',
          strokeWidth: 2,
          opacity: selected ? 0.8 : 0.4,
          transition: 'stroke 150ms ease, opacity 150ms ease',
        }}
      />
      <EdgeLabelRenderer>
        <div
          style={{
            position: 'absolute',
            transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
            pointerEvents: 'none',
          }}
        >
          <IconButton
            onClick={handleDelete}
            onMouseDown={(e) => {
              e.stopPropagation()
              e.preventDefault()
            }}
            onMouseUp={(e) => {
              e.stopPropagation()
              e.preventDefault()
            }}
            size="small"
            sx={{
              width: 12,
              height: 12,
              backgroundColor: '#7d8590',
              color: 'black',
              opacity: selected ? 1 : 0,
              transition: 'opacity 150ms ease, background-color 150ms ease',
              pointerEvents: 'auto',
              willChange: 'opacity, background-color',
              '&:hover': {
                opacity: 1,
                backgroundColor: '#9ca3af',
              },
              '&:active': {
                backgroundColor: '#6b7280',
              },
            }}
          >
            <CloseIcon sx={{ fontSize: 8, fontWeight: 'bold' }} />
          </IconButton>
        </div>
      </EdgeLabelRenderer>
    </>
  )
}

const StepEdge = memo(StepEdgeComponent)

export { StepEdge }
