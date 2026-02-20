import { memo } from 'react'
import { EdgeLabelRenderer, useReactFlow } from '@xyflow/react'
import type { EdgeProps } from '@xyflow/react'
import CloseIcon from '@mui/icons-material/Close'
import IconButton from '@mui/material/IconButton'
import { useTheme } from '@mui/material/styles'
import { CONNECTOR } from './constants'
import { PipeEdgePath } from './PipeEdgePath'
import { computeBezierPath, computeBezierLabel } from './edges/bezierPath'

function StepEdgeComponent(props: EdgeProps) {
  const { id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, selected } = props

  const theme = useTheme()
  const { deleteElements } = useReactFlow()

  const edgePath = computeBezierPath(sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition)
  const { labelX, labelY } = computeBezierLabel(sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition)

  const handleDelete = (event: React.MouseEvent) => {
    event.stopPropagation()
    event.preventDefault()
    void deleteElements({ edges: [{ id }] })
  }

  return (
    <>
      <PipeEdgePath
        edgePath={edgePath}
        selected={selected ?? false}
        interactionWidth={CONNECTOR.INTERACTION_WIDTH}
        sourceX={sourceX}
        sourceY={sourceY}
        targetX={targetX}
        targetY={targetY}
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
              backgroundColor: theme.palette.custom.connectorColor,
              color: theme.palette.custom.screenBg,
              opacity: selected ? 1 : 0,
              transition: 'opacity 150ms ease, background-color 150ms ease',
              pointerEvents: 'auto',
              willChange: 'opacity, background-color',
              '&:hover': {
                opacity: 1,
                backgroundColor: theme.palette.custom.accent,
              },
              '&:active': {
                backgroundColor: theme.palette.custom.accent,
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
