import { memo } from 'react'
import { Handle, Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import type { StepNodeData } from './mappers'
import { CANVAS, STEP_TYPE_COLORS, DEFAULT_STEP_TYPE_COLOR } from './constants'
import { DESIGN } from '@/constants'

function StepNodeComponent({ data, selected }: NodeProps) {
  const nodeData = data as StepNodeData
  const accentColor = STEP_TYPE_COLORS[nodeData.stepType] ?? DEFAULT_STEP_TYPE_COLOR

  return (
    <Box
      sx={{
        width: CANVAS.NODE_WIDTH,
        borderRadius: '12px',
        backgroundColor: 'background.paper',
        border: 2,
        borderColor: selected ? 'primary.main' : 'divider',
        boxShadow: selected
          ? '0 8px 32px rgba(59, 130, 246, 0.15)'
          : '0 4px 24px rgba(0, 0, 0, 0.4)',
        transition: 'border-color 150ms ease, box-shadow 150ms ease',
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 1,
          px: 1.5,
          py: 1,
          backgroundColor: DESIGN.BG_HEADER,
          borderBottom: 1,
          borderColor: 'divider',
        }}
      >
        <Box
          sx={{
            width: 8,
            height: 8,
            borderRadius: '50%',
            backgroundColor: accentColor,
            flexShrink: 0,
          }}
        />
        <Typography
          sx={{
            fontSize: 12,
            fontWeight: 600,
            flex: 1,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            color: 'text.primary',
          }}
        >
          {nodeData.label}
        </Typography>
        <Typography
          sx={{
            fontSize: 9,
            textTransform: 'uppercase',
            color: 'text.secondary',
            letterSpacing: '0.05em',
            fontWeight: 600,
            flexShrink: 0,
          }}
        >
          {nodeData.stepType}
        </Typography>
      </Box>

      {/* Input Handle (left) */}
      <Handle
        type="target"
        position={Position.Left}
        style={{
          width: CANVAS.HANDLE_SIZE,
          height: CANVAS.HANDLE_SIZE,
          background: DEFAULT_STEP_TYPE_COLOR,
          border: `2px solid ${DESIGN.BG_HEADER}`,
        }}
      />

      {/* Output Handle (right) */}
      <Handle
        type="source"
        position={Position.Right}
        style={{
          width: CANVAS.HANDLE_SIZE,
          height: CANVAS.HANDLE_SIZE,
          background: DEFAULT_STEP_TYPE_COLOR,
          border: `2px solid ${DESIGN.BG_HEADER}`,
        }}
      />
    </Box>
  )
}

const StepNode = memo(StepNodeComponent)

export { StepNode }
