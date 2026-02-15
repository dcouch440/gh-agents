import { memo } from 'react'
import { DetailLevel, LOD, PIPE } from './constants'
import { brightenHex, computePipeOpacities } from './pipeEdgeUtils'
import { useCanvasLOD } from './useCanvasLOD'

type PipeEdgePathProps = {
  edgePath: string
  color: string
  selected: boolean
  isProtocol: boolean
  interactionWidth: number
}

function PipeEdgePathComponent({
  edgePath,
  color,
  selected,
  isProtocol,
  interactionWidth,
}: PipeEdgePathProps) {
  const detailLevel = useCanvasLOD()

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
          stroke={color}
          strokeWidth={LOD.MINIMAL_EDGE_WIDTH}
          strokeOpacity={LOD.MINIMAL_EDGE_OPACITY}
          strokeLinecap="round"
        />
      </g>
    )
  }

  const opacities = computePipeOpacities(isProtocol, selected)
  const coreColor = brightenHex(color, PIPE.CORE_BRIGHTEN)
  const showGlow = opacities.glow > 0

  const bodyWidth = isProtocol || selected ? PIPE.BODY_WIDTH : PIPE.BODY_WIDTH_DIM
  const coreWidth = isProtocol || selected ? PIPE.CORE_WIDTH : PIPE.CORE_WIDTH_DIM

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

      {/* Outer glow — subtle halo */}
      {showGlow && (
        <path
          d={edgePath}
          fill="none"
          stroke={color}
          strokeWidth={PIPE.GLOW_WIDTH}
          strokeOpacity={opacities.glow}
          strokeLinecap="round"
        />
      )}

      {/* Pipe body */}
      <path
        d={edgePath}
        fill="none"
        stroke={color}
        strokeWidth={bodyWidth}
        strokeOpacity={opacities.body}
        strokeLinecap="round"
      />

      {/* Inner core highlight */}
      <path
        d={edgePath}
        fill="none"
        stroke={coreColor}
        strokeWidth={coreWidth}
        strokeOpacity={opacities.core}
        strokeLinecap="round"
      />
    </g>
  )
}

const PipeEdgePath = memo(PipeEdgePathComponent)

export { PipeEdgePath }
