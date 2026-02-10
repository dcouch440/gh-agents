import { memo } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import RepeatOutlined from '@mui/icons-material/RepeatOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import SettingsOutlined from '@mui/icons-material/SettingsOutlined'
import type { StepNodeData } from './mappers'
import { nodeDataEqual } from './mappers'
import { CanvasHandle } from './CanvasHandle'
import { CANVAS, DEFAULT_STEP_TYPE_COLOR, PROTOCOL_TYPE_COLORS, GREYSCALE_ACCENT } from './constants'
import { useProtocolHighlight } from './useProtocolHighlight'

const STEP_TYPE_ICONS: Record<string, typeof SettingsOutlined> = {
  single: SmartToyOutlined,
  for_each: RepeatOutlined,
  room: ForumOutlined,
}
const DEFAULT_STEP_TYPE_ICON = SettingsOutlined

function StepNodeComponent({ data, selected }: NodeProps) {
  const theme = useTheme()
  const nodeData = data as StepNodeData
  const highlightMode = useProtocolHighlight(nodeData.protocolStepId)
  const hasProtocol = nodeData.protocolColor !== null
  const accentColor = nodeData.protocolColor ?? GREYSCALE_ACCENT
  const IconComponent = STEP_TYPE_ICONS[nodeData.stepType] ?? DEFAULT_STEP_TYPE_ICON

  const hasInputs = nodeData.upstreamStepNames.length > 0
  const hasTools = nodeData.toolNames.length > 0
  const hasOutput = nodeData.outputSchemaName !== null
  const hasPorts = nodeData.protocolPortNames.length > 0
  const hasBody = hasInputs || hasTools || hasOutput || hasPorts

  const subtitle = nodeData.agentName ? (nodeData.modelId ? `${nodeData.agentName} \u00b7 ${nodeData.modelId}` : nodeData.agentName) : null

  return (
    <Box
      sx={{
        width: CANVAS.NODE_WIDTH,
        borderRadius: '12px',
        backgroundColor: 'background.paper',
        border: 2,
        borderStyle: hasProtocol ? 'dashed' : 'solid',
        borderColor: selected
          ? hasProtocol ? accentColor : 'primary.main'
          : hasProtocol
            ? highlightMode === 'select'
              ? accentColor
              : highlightMode === 'hover'
                ? `${accentColor}80`
                : `${accentColor}50`
            : 'divider',
        boxShadow: selected
          ? `0 8px 32px ${theme.palette.mode === 'dark' ? 'rgba(59, 130, 246, 0.15)' : 'rgba(255, 150, 79, 0.16)'}`
          : highlightMode === 'select'
            ? `0 0 0 1px ${accentColor}40, 0 8px 32px ${accentColor}22`
            : highlightMode === 'hover'
              ? `0 0 0 1px ${accentColor}20, 0 6px 24px ${accentColor}14`
              : `0 4px 24px ${theme.palette.mode === 'dark' ? 'rgba(0, 0, 0, 0.4)' : 'rgba(45, 27, 14, 0.12)'}`,
        transition: 'border-color 150ms ease, box-shadow 150ms ease, border-style 150ms ease',
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <Box
        sx={{
          px: 1.5,
          py: 1,
          backgroundColor: theme.palette.custom.bgHeader,
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
          {nodeData.protocolType !== null ? (
            <Box
              sx={{
                px: 0.75,
                py: 0.25,
                borderRadius: '4px',
                backgroundColor: `${PROTOCOL_TYPE_COLORS[nodeData.protocolType] ?? DEFAULT_STEP_TYPE_COLOR}20`,
                flexShrink: 0,
              }}
            >
              <Typography
                sx={{
                  fontSize: 8,
                  textTransform: 'uppercase',
                  color: PROTOCOL_TYPE_COLORS[nodeData.protocolType] ?? DEFAULT_STEP_TYPE_COLOR,
                  letterSpacing: '0.06em',
                  fontWeight: 700,
                  lineHeight: 1,
                }}
              >
                {nodeData.protocolType}
              </Typography>
            </Box>
          ) : (
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
          )}
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
                      backgroundColor: theme.palette.custom.hoverOverlay,
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
                      backgroundColor: theme.palette.custom.hoverOverlay,
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
          {hasPorts && (
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
                Ports
              </Typography>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
                {nodeData.protocolPortNames.map((name) => (
                  <Box
                    key={name}
                    sx={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 0.5,
                      px: 0.75,
                      py: 0.25,
                      borderRadius: '4px',
                      backgroundColor: `${PROTOCOL_TYPE_COLORS[nodeData.protocolType ?? ''] ?? DEFAULT_STEP_TYPE_COLOR}10`,
                      border: 1,
                      borderColor: `${PROTOCOL_TYPE_COLORS[nodeData.protocolType ?? ''] ?? DEFAULT_STEP_TYPE_COLOR}30`,
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
        </Box>
      )}

      {/* Handles */}
      <CanvasHandle type="target" position={Position.Left} color={accentColor} />
      <CanvasHandle type="source" position={Position.Right} color={accentColor} />
    </Box>
  )
}

const stepNodeEqual = (prev: NodeProps, next: NodeProps): boolean => prev.selected === next.selected && nodeDataEqual(prev.data, next.data)

const StepNode = memo(StepNodeComponent, stepNodeEqual)

export { StepNode }
