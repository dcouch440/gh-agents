import { memo } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import { useTheme } from '@mui/material/styles'
import { CanvasHandle } from '../CanvasHandle'
import { DetailLevel } from '../constants'
import { SUB_WORKFLOW_NODE } from './constants'
import type { SubWorkflowNodeData } from './types'
import { nodeDataEqual } from '../mappers'
import { useCanvasLOD } from '../useCanvasLOD'
import { MinimalNodeShell } from '../MinimalNodeShell'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'

function SubWorkflowNodeComponent({ data, selected }: NodeProps) {
  const theme = useTheme()
  const detailLevel = useCanvasLOD()
  const nodeData = data as SubWorkflowNodeData
  const accentColor = SUB_WORKFLOW_NODE.ACCENT_COLOR

  const highlight = getNodeHighlightStyles({
    selected: selected === true,
    accentColor,
    highlightMode: 'none',
    themeMode: theme.palette.mode,
    variant: 'default',
  })

  if (detailLevel === DetailLevel.MINIMAL) {
    return (
      <Box sx={{ width: '100%', height: '100%' }}>
        <MinimalNodeShell
          label={nodeData.label}
          accentColor={accentColor}
          borderColor={highlight.borderColor}
          boxShadow={highlight.boxShadow}
        />
        <CanvasHandle type="target" position={Position.Left} color={accentColor} variant="small" />
        <CanvasHandle type="source" position={Position.Right} color={accentColor} variant="small" />
      </Box>
    )
  }

  return (
    <Box
      sx={{
        width: '100%',
        height: '100%',
        borderRadius: '10px',
        backgroundColor: 'background.paper',
        border: 2,
        borderColor: highlight.borderColor,
        boxShadow: highlight.boxShadow,
        overflow: 'hidden',
        display: 'flex',
        alignItems: 'center',
        gap: 1,
        px: 1.25,
        cursor: 'grab',
        '&:active': { cursor: 'grabbing' },
      }}
    >
      <Box
        sx={{
          width: 32,
          height: 32,
          borderRadius: '8px',
          backgroundColor: `${accentColor}20`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
        }}
      >
        <SmartToyOutlined sx={{ fontSize: 18, color: accentColor }} />
      </Box>

      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Typography
          sx={{
            fontSize: 12,
            fontWeight: 600,
            color: 'text.primary',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            lineHeight: 1.3,
          }}
        >
          {nodeData.label}
        </Typography>
        {nodeData.templateName !== null && (
          <Typography
            sx={{
              fontSize: 9,
              color: 'text.disabled',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              lineHeight: 1.2,
            }}
          >
            {nodeData.templateName}
          </Typography>
        )}
      </Box>

      <CanvasHandle type="target" position={Position.Left} color={accentColor} variant="small" />
      <CanvasHandle type="source" position={Position.Right} color={accentColor} variant="small" />
    </Box>
  )
}

const subWorkflowNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const SubWorkflowNode = memo(SubWorkflowNodeComponent, subWorkflowNodeEqual)

export { SubWorkflowNode }
