import { Position } from '@xyflow/react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import Tooltip from '@mui/material/Tooltip'
import CircularProgress from '@mui/material/CircularProgress'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import { useTheme } from '@mui/material/styles'
import { useStore, shallow, workflowExecutionStore, stepStreamStore } from '@/stores'
import { CanvasHandle } from '../../CanvasHandle'
import { CANVAS, DEFAULT_STEP_TYPE_COLOR, PROTOCOL_TYPE_COLORS } from '../../constants'
import { NodeHeader, ExecutionStatusBadge, ExecutionProgress, StreamView, ToolActivityFeed, toExecutionStatus } from '../../execution'
import { useWorkshopStepRun } from '../../useWorkshopStepRun'
import { SectionLabel } from '../primitives/SectionLabel'
import { BadgeList } from '../primitives/BadgeList'
import { STEP_TYPE_ICONS, DEFAULT_STEP_TYPE_ICON } from '../registry'
import type { CardNodeData } from '../types'
import type { NodeHighlightOutput } from '../../nodeHighlightStyles'

type CardLayoutProps = {
  nodeId: string
  data: CardNodeData
  selected: boolean
  accentColor: string
  highlight: NodeHighlightOutput
}

function CardLayout({ nodeId, data, accentColor, highlight }: CardLayoutProps) {
  const theme = useTheme()
  const IconComponent = STEP_TYPE_ICONS[data.stepType] ?? DEFAULT_STEP_TYPE_ICON

  // Execution state
  const stepExec = useStore(workflowExecutionStore.store, workflowExecutionStore.selectStepState(nodeId))
  const execStatus = toExecutionStatus(stepExec?.status)
  const isExecuting = execStatus !== 'idle'

  // Streaming state
  const sources = useStore(stepStreamStore.store, stepStreamStore.selectSourcesForStep(nodeId), shallow)
  const activeSource = sources.length > 0 ? sources[0] : null
  const hasStream = activeSource !== null && (activeSource.streamBuffer !== '' || activeSource.toolUses.length > 0)

  // Workshop run
  const showRunButton = data.stepType !== 'context' && data.stepType !== 'input'
  const { status: workshopStatus, handleRun } = useWorkshopStepRun(nodeId)
  const workshopRunning = workshopStatus === 'running' || workshopStatus === 'initializing'

  const hasInputs = data.upstreamStepNames.length > 0
  const hasTools = data.toolNames.length > 0
  const hasOutput = data.outputSchemaName !== null
  const hasPorts = data.protocolPortNames.length > 0
  const hasBody = hasInputs || hasTools || hasOutput || hasPorts

  const subtitle = data.agentName ? (data.modelId ? `${data.agentName} \u00b7 ${data.modelId}` : data.agentName) : null
  const portColor = PROTOCOL_TYPE_COLORS[data.protocolType ?? ''] ?? DEFAULT_STEP_TYPE_COLOR

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
          title={data.label}
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
            ) : data.protocolType !== null ? (
              <Box
                sx={{
                  px: 0.75,
                  py: 0.25,
                  borderRadius: '4px',
                  backgroundColor: `${PROTOCOL_TYPE_COLORS[data.protocolType] ?? DEFAULT_STEP_TYPE_COLOR}20`,
                  flexShrink: 0,
                }}
              >
                <Typography
                  sx={{
                    fontSize: 8,
                    textTransform: 'uppercase',
                    color: PROTOCOL_TYPE_COLORS[data.protocolType] ?? DEFAULT_STEP_TYPE_COLOR,
                    letterSpacing: '0.06em',
                    fontWeight: 700,
                    lineHeight: 1,
                  }}
                >
                  {data.protocolType}
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
                {data.stepType}
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
              <BadgeList items={data.upstreamStepNames} />
            </Box>
          )}
          {hasTools && (
            <Box>
              <SectionLabel label="Tools" />
              <BadgeList items={data.toolNames} />
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
                {data.outputSchemaName}
              </Typography>
            </Box>
          )}
          {hasPorts && (
            <Box>
              <SectionLabel label="Ports" />
              <BadgeList
                items={data.protocolPortNames}
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

export { CardLayout }
export type { CardLayoutProps }
