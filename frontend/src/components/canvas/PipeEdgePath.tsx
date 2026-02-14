import { memo } from 'react'
import { PIPE } from './constants'
import { brightenHex, computePipeOpacities } from './pipeEdgeUtils'
import './edgeFlow.css'

type PipeEdgePathProps = {
  edgePath: string
  color: string
  selected: boolean
  isProtocol: boolean
  animationDirection: 'normal' | 'reverse'
  interactionWidth: number
}

function PipeEdgePathComponent({
  edgePath,
  color,
  selected,
  isProtocol,
  animationDirection,
  interactionWidth,
}: PipeEdgePathProps) {
  const opacities = computePipeOpacities(isProtocol, selected)
  const coreColor = brightenHex(color, PIPE.CORE_BRIGHTEN)
  const showGlow = opacities.glow > 0
  const showParticles = opacities.particle > 0

  const bodyWidth = isProtocol || selected ? PIPE.BODY_WIDTH : PIPE.BODY_WIDTH_DIM
  const coreWidth = isProtocol || selected ? PIPE.CORE_WIDTH : PIPE.CORE_WIDTH_DIM

  const animationName = animationDirection === 'reverse' ? 'pipeFlowReverse' : 'pipeFlow'

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

      {/* Outer glow — wide semi-transparent stroke, no filter */}
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

      {/* Flowing energy particles */}
      {showParticles && (
        <path
          d={edgePath}
          fill="none"
          stroke={color}
          strokeWidth={PIPE.PARTICLE_WIDTH}
          strokeDasharray={PIPE.PARTICLE_DASH}
          strokeLinecap="round"
          strokeOpacity={opacities.particle}
          style={{
            animation: `${animationName} ${PIPE.FLOW_DURATION} linear infinite`,
          }}
        />
      )}
    </g>
  )
}

const PipeEdgePath = memo(PipeEdgePathComponent)

export { PipeEdgePath }
