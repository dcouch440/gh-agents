import { memo } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import Tooltip from '@mui/material/Tooltip'
import CircularProgress from '@mui/material/CircularProgress'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import { useTheme } from '@mui/material/styles'
import { useStore, shallow, workflowExecutionStore, stepStreamStore } from '@/stores'
import type { StepNodeData } from '../mappers'
import { nodeDataEqual } from '../mappers'
import { CanvasHandle } from '../CanvasHandle'
import { CANVAS, DEFAULT_STEP_TYPE_COLOR, STEP_TYPE_COLORS, PROTOCOL_TYPE_COLORS, GREYSCALE_ACCENT, DetailLevel } from '../constants'
import { CanvasNodeKind } from '../canvasKinds'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'
import { useCanvasLOD } from '../useCanvasLOD'
import { MinimalNodeShell } from '../MinimalNodeShell'
import { NodeHeader, ExecutionStatusBadge, ExecutionProgress, StreamView, ToolActivityFeed, toExecutionStatus } from '../execution'
import { useWorkshopStepRun } from '../useWorkshopStepRun'
import { SectionLabel } from './SectionLabel'
import { BadgeList } from './BadgeList'
import { STEP_TYPE_ICONS, DEFAULT_STEP_TYPE_ICON } from './constants'

function StepNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const detailLevel = useCanvasLOD()
  const nodeData = data as StepNodeData
  const highlightMode = useProtocolHighlight(CanvasNodeKind.STEP, id, nodeData.protocolStepId)
  const rawAccent = STEP_TYPE_COLORS[nodeData.stepType] ?? GREYSCALE_ACCENT
  const accentColor = theme.palette.mode === 'light' ? theme.palette.custom.accent : rawAccent
  const IconComponent = STEP_TYPE_ICONS[nodeData.stepType] ?? DEFAULT_STEP_TYPE_ICON

  // Execution state
  const stepExec = useStore(workflowExecutionStore.store, workflowExecutionStore.selectStepState(id))
  const execStatus = toExecutionStatus(stepExec?.status)
  const isExecuting = execStatus !== 'idle'

  // Streaming state
  const sources = useStore(stepStreamStore.store, stepStreamStore.selectSourcesForStep(id), shallow)
  const activeSource = sources.length > 0 ? sources[0] : null
  const hasStream = activeSource !== null && (activeSource.streamBuffer !== '' || activeSource.toolUses.length > 0)

  // Workshop run
  const showRunButton = nodeData.stepType !== 'context' && nodeData.stepType !== 'input'
  const { status: workshopStatus, handleRun } = useWorkshopStepRun(id)
  const workshopRunning = workshopStatus === 'running' || workshopStatus === 'initializing'
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
        borderRadius: '8px',
        backgroundColor: theme.palette.custom.screenBg,
        border: 1,
        borderColor: highlight.borderColor,
        boxShadow: highlight.boxShadow,
        transition: 'border-color 150ms ease',
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <Box
        sx={{
          px: 0,
          py: 1,
          backgroundColor: theme.palette.custom.screenBg,
          borderBottom: hasBody ? 1 : 0,
          borderColor: 'divider',
        }}
      >
        <NodeHeader
          icon={<IconComponent sx={{ fontSize: 14, color: accentColor }} />}
          title={nodeData.label}
          subtitle={subtitle}
          accentColor={accentColor}
          size="compact"
          actions={showRunButton ? (
            <Tooltip title={workshopRunning ? 'Running...' : 'Run step'} placement="top">
              <span>
                <IconButton className="nodrag" onClick={handleRun} disabled={workshopRunning} size="small" sx={{ flexShrink: 0, width: 24, height: 24, color: 'text.secondary', '&:hover': { color: 'text.primary' } }}>
                  {workshopRunning
                    ? <CircularProgress size={10} thickness={5} sx={{ color: 'text.secondary' }} />
                    : <PlayArrowOutlined sx={{ fontSize: 14 }} />}
                </IconButton>
              </span>
            </Tooltip>
          ) : undefined}
          badge={
            isExecuting ? (
              <ExecutionStatusBadge status={execStatus} />
            ) : nodeData.protocolType !== null ? (
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
            )
          }
        />
      </Box>

      {/* For-each progress */}
      {stepExec?.forEachProgress !== null && stepExec?.forEachProgress !== undefined && (
        <Box sx={{ px: 1.5, py: 0.5, borderBottom: 1, borderColor: 'divider' }}>
          <ExecutionProgress
            completed={stepExec.forEachProgress.completed}
            total={stepExec.forEachProgress.total}
            label="Items"
            accentColor={accentColor}
          />
        </Box>
      )}

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

      {/* Live stream */}
      {hasStream && activeSource && (
        <Box
          className="nowheel nodrag nopan"
          sx={{ borderTop: 1, borderColor: 'divider', maxHeight: 160, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}
        >
          <Box sx={{ flex: 1, minHeight: 0, px: 1, py: 0.5 }}>
            <StreamView
              content={activeSource.streamBuffer}
              status={activeSource.status === 'completed' || activeSource.status === 'failed' ? activeSource.status : 'running'}
              error={activeSource.error}
              maxHeight={120}
            />
          </Box>
          {activeSource.toolUses.length > 0 && (
            <Box sx={{ px: 1, py: 0.5, borderTop: 1, borderColor: 'divider', flexShrink: 0 }}>
              <ToolActivityFeed tools={activeSource.toolUses} compact />
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
