import { memo } from 'react'
import { getBezierPath } from '@xyflow/react'
import type { EdgeProps } from '@xyflow/react'
import { PIPE } from './constants'
import { PipeEdgePath } from './PipeEdgePath'
import { NOTES_NODE } from './NotesNode'

function NotesEdgeComponent(props: EdgeProps) {
  const { sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition } = props

  const [edgePath] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
  })

  return (
    <PipeEdgePath
      edgePath={edgePath}
      color={NOTES_NODE.ACCENT_COLOR}
      selected={false}
      isProtocol={true}
      animationDirection="reverse"
      interactionWidth={PIPE.INTERACTION_WIDTH}
    />
  )
}

const NotesEdge = memo(NotesEdgeComponent)

export { NotesEdge }
