import { Handle } from '@xyflow/react'
import type { Position } from '@xyflow/react'
import { useTheme } from '@mui/material/styles'
import { CANVAS } from './constants'

type CanvasHandleVariant = 'standard' | 'small' | 'passive'

type CanvasHandleProps = {
  type: 'source' | 'target'
  position: Position
  color: string
  variant?: CanvasHandleVariant
  id?: string
  style?: React.CSSProperties
}

function CanvasHandle({ type, position, color, variant = 'standard', id, style: styleProp }: CanvasHandleProps) {
  const theme = useTheme()
  const size = variant === 'standard' ? CANVAS.HANDLE_SIZE : CANVAS.HANDLE_SIZE_SMALL

  return (
    <Handle
      type={type}
      position={position}
      id={id}
      style={{
        width: size,
        height: size,
        background: color,
        border: `${CANVAS.HANDLE_BORDER_WIDTH}px solid ${theme.palette.custom.bgHeader}`,
        ...(variant === 'passive' ? { pointerEvents: 'none' as const } : {}),
        ...styleProp,
      }}
    />
  )
}

export { CanvasHandle }
export type { CanvasHandleVariant, CanvasHandleProps }
