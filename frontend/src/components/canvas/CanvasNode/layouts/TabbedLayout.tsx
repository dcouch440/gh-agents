import { useMemo, useState, useCallback, useEffect } from 'react'
import { Position } from '@xyflow/react'
import Box from '@mui/material/Box'
import InfoOutlined from '@mui/icons-material/InfoOutlined'
import IconButton from '@mui/material/IconButton'
import Tooltip from '@mui/material/Tooltip'
import Typography from '@mui/material/Typography'
import OpenInFullOutlined from '@mui/icons-material/OpenInFullOutlined'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import CircularProgress from '@mui/material/CircularProgress'
import AssignmentOutlined from '@mui/icons-material/AssignmentOutlined'
import { useStore, shareStore, dispatchStore, workflowStore } from '@/stores'
import { CanvasFormNode } from '../../CanvasFormNode'
import { CanvasHandle } from '../../CanvasHandle'
import { NOTES_ACCENT } from '../../constants'
import { HighlightMode } from '../../canvasKinds'
import { ProtocolBadge } from '../../ProtocolBadge'
import { useEnterFocusMode } from '../../useEnterFocusMode'
import { useWorkshopStepRun } from '../../useWorkshopStepRun'
import { NodeHeader, ExecutionStatusBadge } from '../../execution'
import { SharePickerPanel } from '../../SharePickerPanel'
import { Archetype, ARCHETYPE_CONFIGS, AGENT_CONSTRAINTS } from '../registry'
import { useDynamicNodeExecution } from '../hooks'
import { useStepStoreData } from '../hooks'
import { buildStepTabs } from '../tabs/buildStepTabs'
import { AgentStreamTab } from '../tabs/AgentStreamTab'
import { AgentInfoTab } from '../tabs/AgentInfoTab'
import type { TabbedNodeData, AgentNodeData } from '../types'

type TabbedLayoutProps = {
  nodeId: string
  data: TabbedNodeData | AgentNodeData
  selected: boolean
  accentColor: string
  highlightMode: HighlightMode
}

function TabbedLayout({ nodeId, data, selected, accentColor, highlightMode }: TabbedLayoutProps) {
  const isAgent = data.variant === 'agent'

  const [activeTabId, setActiveTabId] = useState(isAgent ? 'info' : 'chat')

  // --- Agent-specific fields (safe: only read when isAgent is true) ---
  const rosterAgentId = isAgent ? data.rosterAgentId : null
  const roleDescription = isAgent ? (data.roleDescription ?? '') : ''
  const capabilities = isAgent ? data.capabilities : null

  // --- Execution ---
  const protocolStepId = data.protocolStepId
  const { isExecuting, resolvedExecStatus, agentSourceStatus, stepExecStatus } =
    useDynamicNodeExecution(nodeId, isAgent, rosterAgentId, protocolStepId)
  const { stepIssues } = useStepStoreData(nodeId)
  const activeDispatch = useStore(dispatchStore.store, dispatchStore.selectActiveForStep(nodeId))
  const questionState = useStore(workflowStore.store, workflowStore.selectStepQuestionState(nodeId))

  // --- Share mode (steps only) ---
  const shareActive = useStore(shareStore.store, isAgent ? () => false : shareStore.selectActive)
  const shareSourceId = useStore(shareStore.store, isAgent ? () => null : shareStore.selectSourceStepId)
  const pendingChatFocus = useStore(shareStore.store, isAgent ? () => null : shareStore.selectPendingChatFocus)
  const isShareSource = shareActive && shareSourceId === nodeId

  useEffect(() => {
    if (pendingChatFocus === nodeId) {
      queueMicrotask(() => { setActiveTabId('chat') })
      shareStore.clearPendingChatFocus()
    }
  }, [pendingChatFocus, nodeId])

  useEffect(() => {
    if (isAgent && agentSourceStatus === 'running') {
      queueMicrotask(() => { setActiveTabId('stream') })
    } else if (!isAgent && stepExecStatus === 'running') {
      queueMicrotask(() => { setActiveTabId('live') })
    }
  }, [isAgent, agentSourceStatus, stepExecStatus])

  const shareOverlay = isShareSource ? <SharePickerPanel stepId={nodeId} /> : undefined
  const effectiveHighlight = shareActive && !isShareSource ? HighlightMode.HOVER : highlightMode

  // --- Build tabs ---
  const archetype = isAgent ? Archetype.AGENT : data.variant === 'workforce' ? Archetype.WORKFORCE : data.variant === 'manager' ? Archetype.MANAGER : data.variant === 'room' ? Archetype.ROOM : Archetype.BLANK
  const tabs = useMemo(() => isAgent
    ? [
        { id: 'stream', icon: AssignmentOutlined, tooltip: 'Run Results', content: <AgentStreamTab rosterAgentId={rosterAgentId ?? ''} protocolStepId={protocolStepId} agentName={data.label} /> },
        { id: 'info', icon: InfoOutlined, tooltip: 'Info', content: <AgentInfoTab roleDescription={roleDescription} capabilities={capabilities ?? []} /> },
      ]
    : buildStepTabs({
        stepId: nodeId,
        archetype,
        includeLiveStream: true,
      }),
  [isAgent, nodeId, data, rosterAgentId, protocolStepId, roleDescription, capabilities, archetype])

  // --- Header ---
  const config = ARCHETYPE_CONFIGS[archetype]
  const enterFocusMode = useEnterFocusMode()
  const handleEnterFocusMode = useCallback(() => { enterFocusMode(nodeId) }, [enterFocusMode, nodeId])
  const { status: workshopStatus, handleRun: handleWorkshopRun } = useWorkshopStepRun(nodeId)

  // --- Handles ---
  const agentHandles = (
    <>
      <CanvasHandle type="target" position={Position.Bottom} id="agent-input" color={accentColor} variant="passive" />
      <CanvasHandle type="source" position={Position.Top} id="agent-output" color={accentColor} variant="passive" />
    </>
  )

  const stepExtraHandles = data.variant === 'workforce' ? (
    <CanvasHandle type="source" position={Position.Top} id="agents" color={accentColor} variant="passive" />
  ) : undefined

  const protocolHandles = (
    <>
      <CanvasHandle type="target" position={Position.Left} color={accentColor} style={{ top: '33%' }} />
      <CanvasHandle type="source" position={Position.Right} color={accentColor} />
      {stepExtraHandles}
    </>
  )

  // --- Header badge ---
  const DISPATCH_COLOR = '#8b5cf6'
  const QUESTION_COLOR = '#f59e0b'
  const headerBadge = isExecuting ? (
    <ExecutionStatusBadge status={resolvedExecStatus} />
  ) : activeDispatch ? (
    <ProtocolBadge color={DISPATCH_COLOR} label="Dispatching..." animated />
  ) : questionState?.question_text ? (
    <ProtocolBadge color={QUESTION_COLOR} label="Has Question" animated />
  ) : stepIssues.length > 0 ? (
    <Tooltip
      title={
        <Box sx={{ py: 0.5 }}>
          {stepIssues.map((issue, i) => (
            <Typography key={i} sx={{ fontSize: 12, lineHeight: 1.4 }}>{issue.description}</Typography>
          ))}
        </Box>
      }
      arrow
      placement="top"
    >
      <span>
        <ProtocolBadge color={NOTES_ACCENT} label={`${stepIssues.length} Issue${stepIssues.length > 1 ? 's' : ''}`} animated />
      </span>
    </Tooltip>
  ) : data.variant !== 'blank' ? (
    <ProtocolBadge color={accentColor} label={config.label} animated />
  ) : undefined

  const workshopRunning = workshopStatus === 'running' || workshopStatus === 'initializing'

  const headerActions = isAgent ? undefined : (
    <Box sx={{ display: 'flex', gap: 0.25, alignItems: 'center' }}>
      <Tooltip title={workshopRunning ? 'Running...' : 'Run step'} placement="top">
        <span>
          <IconButton className="nodrag" onClick={handleWorkshopRun} disabled={workshopRunning} size="small" sx={{ flexShrink: 0, width: 28, height: 28, color: 'text.secondary', '&:hover': { color: 'text.primary' } }}>
            {workshopRunning
              ? <CircularProgress size={12} thickness={5} sx={{ color: 'text.secondary' }} />
              : <PlayArrowOutlined sx={{ fontSize: 16 }} />}
          </IconButton>
        </span>
      </Tooltip>
      <IconButton className="nodrag" onClick={handleEnterFocusMode} size="small" sx={{ flexShrink: 0, width: 28, height: 28, color: 'text.secondary', '&:hover': { color: 'text.primary' } }}>
        <OpenInFullOutlined sx={{ fontSize: 16 }} />
      </IconButton>
    </Box>
  )

  const IconComponent = config.icon
  const headerElement = (
    <NodeHeader
      icon={<IconComponent sx={{ fontSize: isAgent ? 18 : 20, color: accentColor }} />}
      title={data.label}
      subtitle={null}
      accentColor={accentColor}
      size={isAgent ? 'standard' : 'large'}
      badge={headerBadge}
      actions={headerActions}
    />
  )

  return (
    <CanvasFormNode
      nodeId={nodeId}
      header={headerElement}
      headerHeight={isAgent ? 52 : undefined}
      tabs={tabs}
      activeTabId={activeTabId}
      onTabChange={setActiveTabId}
      selected={selected}
      accentColor={accentColor}
      highlightMode={effectiveHighlight}
      overlay={shareOverlay}
      {...(isAgent
        ? { handles: agentHandles, constraints: AGENT_CONSTRAINTS }
        : data.variant === 'manager'
          ? { handles: <></> }
          : data.variant === 'workforce'
            ? { handles: protocolHandles }
            : { extraHandles: stepExtraHandles }
      )}
    />
  )
}

export { TabbedLayout }
export type { TabbedLayoutProps }
