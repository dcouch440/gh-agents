import { memo } from 'react'
import { Handle, Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import RepeatOutlined from '@mui/icons-material/RepeatOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import SettingsOutlined from '@mui/icons-material/SettingsOutlined'
import type { StepNodeData } from './mappers'
import { CANVAS, STEP_TYPE_COLORS, DEFAULT_STEP_TYPE_COLOR } from './constants'
import { DESIGN } from '@/constants'

const STEP_TYPE_ICONS: Record<string, typeof SettingsOutlined> = {
  single: SmartToyOutlined,
  for_each: RepeatOutlined,
  room: ForumOutlined,
}
const DEFAULT_STEP_TYPE_ICON = SettingsOutlined

function StepNodeComponent({ data, selected }: NodeProps) {
  const nodeData = data as StepNodeData
  const accentColor = STEP_TYPE_COLORS[nodeData.stepType] ?? DEFAULT_STEP_TYPE_COLOR
  const IconComponent = STEP_TYPE_ICONS[nodeData.stepType] ?? DEFAULT_STEP_TYPE_ICON

  const hasInputs = nodeData.upstreamStepNames.length > 0
  const hasTools = nodeData.toolNames.length > 0
  const hasOutput = nodeData.outputSchemaName !== null
  const hasBody = hasInputs || hasTools || hasOutput

  const subtitle = nodeData.agentName
    ? nodeData.modelId
      ? `${nodeData.agentName} \u00b7 ${nodeData.modelId}`
      : nodeData.agentName
    : null

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
          px: 1.5,
          py: 1,
          backgroundColor: DESIGN.BG_HEADER,
          borderBottom: hasBody ? 1 : 0,
          borderColor: 'divider',
        }}
      >
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <Box
            sx={{
              width: 24,
              height: 24,
              borderRadius: '6px',
              backgroundColor: `${accentColor}25`,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flexShrink: 0,
            }}
          >
            <IconComponent sx={{ fontSize: 14, color: accentColor }} />
          </Box>
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
        {subtitle !== null && (
          <Typography
            sx={{
              fontSize: 10,
              color: 'text.secondary',
              mt: 0.25,
              pl: 4,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {subtitle}
          </Typography>
        )}
      </Box>

      {/* Body — conditional */}
      {hasBody && (
        <Box sx={{ px: 1.5, py: 1, display: 'flex', flexDirection: 'column', gap: 0.75 }}>
          {hasInputs && (
            <Box>
              <Typography
                sx={{
                  fontSize: 8,
                  fontWeight: 600,
                  textTransform: 'uppercase',
                  color: 'text.disabled',
                  letterSpacing: '0.06em',
                  lineHeight: 1,
                  mb: 0.5,
                }}
              >
                Inputs
              </Typography>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
                {nodeData.upstreamStepNames.map((name, idx) => (
                  <Box
                    key={idx}
                    sx={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 0.5,
                      px: 0.75,
                      py: 0.25,
                      borderRadius: '4px',
                      backgroundColor: 'rgba(255,255,255,0.03)',
                      border: 1,
                      borderColor: 'divider',
                      fontSize: 10,
                      color: 'text.secondary',
                      lineHeight: 1.3,
                      maxWidth: '100%',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {name}
                  </Box>
                ))}
              </Box>
            </Box>
          )}
          {hasTools && (
            <Box>
              <Typography
                sx={{
                  fontSize: 8,
                  fontWeight: 600,
                  textTransform: 'uppercase',
                  color: 'text.disabled',
                  letterSpacing: '0.06em',
                  lineHeight: 1,
                  mb: 0.5,
                }}
              >
                Tools
              </Typography>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
                {nodeData.toolNames.map((name, idx) => (
                  <Box
                    key={idx}
                    sx={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 0.5,
                      px: 0.75,
                      py: 0.25,
                      borderRadius: '4px',
                      backgroundColor: 'rgba(255,255,255,0.03)',
                      border: 1,
                      borderColor: 'divider',
                      fontSize: 10,
                      color: 'text.secondary',
                      lineHeight: 1.3,
                      maxWidth: '100%',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {name}
                  </Box>
                ))}
              </Box>
            </Box>
          )}
          {hasOutput && (
            <Box>
              <Typography
                sx={{
                  fontSize: 8,
                  fontWeight: 600,
                  textTransform: 'uppercase',
                  color: 'text.disabled',
                  letterSpacing: '0.06em',
                  lineHeight: 1,
                  mb: 0.25,
                }}
              >
                Output
              </Typography>
              <Typography
                sx={{
                  fontSize: 10,
                  color: 'text.secondary',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                {nodeData.outputSchemaName}
              </Typography>
            </Box>
          )}
        </Box>
      )}

      {/* Handles */}
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
