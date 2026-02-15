import { memo } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import type { StepNodeData } from '../mappers'
import { nodeDataEqual } from '../mappers'
import { CanvasHandle } from '../CanvasHandle'
import { CANVAS, DEFAULT_STEP_TYPE_COLOR, STEP_TYPE_COLORS, PROTOCOL_TYPE_COLORS, GREYSCALE_ACCENT, DetailLevel } from '../constants'
import { CanvasNodeKind } from '../canvasKinds'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'
import { useCanvasLOD } from '../useCanvasLOD'
import { MinimalNodeShell } from '../MinimalNodeShell'
import { SectionLabel } from './SectionLabel'
import { BadgeList } from './BadgeList'
import { STEP_TYPE_ICONS, DEFAULT_STEP_TYPE_ICON } from './constants'

function StepNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const detailLevel = useCanvasLOD()
  const nodeData = data as StepNodeData
  const highlightMode = useProtocolHighlight(CanvasNodeKind.STEP, id, nodeData.protocolStepId)
  const accentColor = STEP_TYPE_COLORS[nodeData.stepType] ?? GREYSCALE_ACCENT
  const IconComponent = STEP_TYPE_ICONS[nodeData.stepType] ?? DEFAULT_STEP_TYPE_ICON
  const highlight = getNodeHighlightStyles({
    selected: selected === true,
    accentColor,
    highlightMode,
    themeMode: theme.palette.mode,
  })

  const hasInputs = nodeData.upstreamStepNames.length > 0
  const hasTools = nodeData.toolNames.length > 0
  const hasOutput = nodeData.outputSchemaName !== null
  const hasPorts = nodeData.protocolPortNames.length > 0
  const hasBody = hasInputs || hasTools || hasOutput || hasPorts

  const subtitle = nodeData.agentName ? (nodeData.modelId ? `${nodeData.agentName} \u00b7 ${nodeData.modelId}` : nodeData.agentName) : null

  const portColor = PROTOCOL_TYPE_COLORS[nodeData.protocolType ?? ''] ?? DEFAULT_STEP_TYPE_COLOR

  if (detailLevel === DetailLevel.MINIMAL) {
    return (
      <Box sx={{ width: CANVAS.NODE_WIDTH }}>
        <MinimalNodeShell
          label={nodeData.label}
          accentColor={accentColor}
          borderColor={highlight.borderColor}
          boxShadow={highlight.boxShadow}
        />
        <CanvasHandle type="target" position={Position.Left} color={accentColor} />
        <CanvasHandle type="source" position={Position.Right} color={accentColor} />
      </Box>
    )
  }

  return (
    <Box
      sx={{
        width: CANVAS.NODE_WIDTH,
        borderRadius: '12px',
        backgroundColor: 'background.paper',
        border: 2,
        borderColor: highlight.borderColor,
        boxShadow: highlight.boxShadow,
        transition: 'border-color 150ms ease, box-shadow 150ms ease',
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
              <SectionLabel label="Inputs" />
              <BadgeList items={nodeData.upstreamStepNames} />
            </Box>
          )}
          {hasTools && (
            <Box>
              <SectionLabel label="Tools" />
              <BadgeList items={nodeData.toolNames} />
            </Box>
          )}
          {hasOutput && (
            <Box>
              <SectionLabel label="Output" />
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
              <SectionLabel label="Ports" />
              <BadgeList
                items={nodeData.protocolPortNames}
                badgeSx={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 0.5,
                  px: 0.75,
                  py: 0.25,
                  borderRadius: '4px',
                  backgroundColor: `${portColor}10`,
                  border: 1,
                  borderColor: `${portColor}30`,
                  fontSize: 10,
                  color: 'text.secondary',
                  lineHeight: 1.3,
                  maxWidth: '100%',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              />
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
