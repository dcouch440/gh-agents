import { memo } from 'react'
import { useTheme } from '@mui/material/styles'
import { DetailLevel, LOD, CONNECTOR } from './constants'
import { useCanvasLOD } from './useCanvasLOD'

type PipeEdgePathProps = {
  edgePath: string
  color?: string
  selected: boolean
  interactionWidth: number
  sourceX: number
  sourceY: number
  targetX: number
  targetY: number
}

function PipeEdgePathComponent({
  edgePath,
  color,
  selected,
  interactionWidth,
  sourceX,
  sourceY,
  targetX,
  targetY,
}: PipeEdgePathProps) {
  const theme = useTheme()
  const detailLevel = useCanvasLOD()
  const resolvedColor = color ?? theme.palette.custom.connectorColor

  if (detailLevel === DetailLevel.MINIMAL) {
    return (
      <g>
        <path
          d={edgePath}
          fill="none"
          strokeOpacity={0}
          strokeWidth={interactionWidth}
          className="react-flow__edge-interaction"
        />
        <path
          d={edgePath}
          fill="none"
          stroke={resolvedColor}
          strokeWidth={LOD.MINIMAL_EDGE_WIDTH}
          strokeOpacity={LOD.MINIMAL_EDGE_OPACITY}
          strokeLinecap="round"
        />
      </g>
    )
  }

  const strokeColor = selected ? theme.palette.custom.accent : resolvedColor

  return (
    <g>
      {/* Interaction hit area */}
      <path
        d={edgePath}
        fill="none"
        strokeOpacity={0}
        strokeWidth={interactionWidth}
        className="react-flow__edge-interaction"
      />

      {/* Dotted connector */}
      <path
        d={edgePath}
        fill="none"
        stroke={strokeColor}
        strokeWidth={CONNECTOR.STROKE_WIDTH}
        strokeDasharray={CONNECTOR.DASH_ARRAY}
        strokeLinecap="round"
      />

      {/* Endpoint dots */}
      <circle cx={sourceX} cy={sourceY} r={CONNECTOR.DOT_RADIUS} fill={strokeColor} />
      <circle cx={targetX} cy={targetY} r={CONNECTOR.DOT_RADIUS} fill={strokeColor} />
    </g>
  )
}

const PipeEdgePath = memo(PipeEdgePathComponent)

export { PipeEdgePath }
export type { PipeEdgePathProps }
