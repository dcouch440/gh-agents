import { memo } from 'react'
import { EdgeLabelRenderer, getBezierPath, useReactFlow } from '@xyflow/react'
import type { EdgeProps } from '@xyflow/react'
import CloseIcon from '@mui/icons-material/Close'
import IconButton from '@mui/material/IconButton'
import { useTheme } from '@mui/material/styles'
import { PIPE } from './constants'
import { PipeEdgePath } from './PipeEdgePath'

type StepEdgeNodeData = {
  sourceColor: string
  isProtocolEdge: boolean
}

function StepEdgeComponent(props: EdgeProps) {
  const { id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, selected, data } = props
  const rawData = data as Partial<StepEdgeNodeData> | undefined
  const sourceColor = rawData?.sourceColor ?? null
  const isProtocolEdge = rawData?.isProtocolEdge ?? false

  const theme = useTheme()
  const { deleteElements } = useReactFlow()

  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
  })

  const color = selected
    ? theme.palette.primary.main
    : sourceColor ?? theme.palette.text.secondary

  const handleDelete = (event: React.MouseEvent) => {
    event.stopPropagation()
    event.preventDefault()
    void deleteElements({ edges: [{ id }] })
  }

  return (
    <>
      <PipeEdgePath
        edgePath={edgePath}
        color={color}
        selected={selected ?? false}
        isProtocol={isProtocolEdge}
        interactionWidth={PIPE.INTERACTION_WIDTH}
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
              backgroundColor: theme.palette.text.secondary,
              color: theme.palette.background.paper,
              opacity: selected ? 1 : 0,
              transition: 'opacity 150ms ease, background-color 150ms ease',
              pointerEvents: 'auto',
              willChange: 'opacity, background-color',
              '&:hover': {
                opacity: 1,
                backgroundColor: theme.palette.text.disabled,
              },
              '&:active': {
                backgroundColor: theme.palette.text.secondary,
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
