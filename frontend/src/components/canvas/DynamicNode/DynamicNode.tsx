import { memo, useState, useCallback, useEffect } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'

import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import InfoOutlined from '@mui/icons-material/InfoOutlined'
import IconButton from '@mui/material/IconButton'
import Tooltip from '@mui/material/Tooltip'
import Typography from '@mui/material/Typography'
import OpenInFullOutlined from '@mui/icons-material/OpenInFullOutlined'
import StreamOutlined from '@mui/icons-material/StreamOutlined'
import { useStore, canvasStore, shareStore } from '@/stores'
import { CanvasFormNode } from '../CanvasFormNode'
import { CanvasHandle } from '../CanvasHandle'
import { DOCUMENT_NODE } from '../DocumentNode'
import { DetailLevel, NOTES_ACCENT } from '../constants'
import { nodeDataEqual } from '../mappers'
import { HighlightMode, CanvasNodeKind } from '../canvasKinds'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { useCanvasLOD } from '../useCanvasLOD'
import { MinimalNodeShell } from '../MinimalNodeShell'
import { ProtocolBadge } from '../ProtocolBadge'
import { useEnterFocusMode } from '../useEnterFocusMode'
import { NodeHeader, ExecutionStatusBadge } from '../execution'
import { SharePickerPanel } from '../SharePickerPanel'
import { Archetype, ARCHETYPE_CONFIGS, AGENT_CONSTRAINTS } from './archetypes'
import type { Archetype as ArchetypeType } from './archetypes'
import { useDynamicNodeExecution } from './useDynamicNodeExecution'
import { useStepStoreData } from './useStepStoreData'
import { useDocumentActions } from './useDocumentActions'
import { buildStepTabs } from './buildStepTabs'
import { resolveSubtitle } from './resolveSubtitle'
import { AgentStreamTab } from './tabs/AgentStreamTab'
import { AgentInfoTab } from './tabs/AgentInfoTab'

type DynamicNodeData = {
  kind: CanvasNodeKind
  archetype: ArchetypeType
  label: string
  description: string
  documentNames: string[]
  rosterNames: string[]
  roomId: string | null
  upstreamStepNames: string[]
  promptValue: string
  modelId: string | null
  agentName: string | null
  // Agent-specific (archetype === 'agent')
  rosterAgentId: string | null
  roleDescription: string | null
  capabilities: string[]
  parentStepName: string | null
  protocolStepId: string | null
}

function DynamicNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const detailLevel = useCanvasLOD()
  const nodeData = data as DynamicNodeData
  const isAgent = nodeData.archetype === Archetype.AGENT

  const config = ARCHETYPE_CONFIGS[nodeData.archetype]
  const accentColor = config.color

  const [activeTabId, setActiveTabId] = useState(isAgent ? 'info' : 'chat')

  // --- Extracted hooks ---
  const { isExecuting, resolvedExecStatus, agentSourceStatus, stepExecStatus } =
    useDynamicNodeExecution(id, isAgent, nodeData.rosterAgentId)
  const { documentDefs, roomStepMembers, stepIssues } = useStepStoreData(id)
  const documentActions = useDocumentActions(id)

  // --- Highlight ---
  const protocolHighlight = useProtocolHighlight(
    isAgent ? CanvasNodeKind.AGENT : CanvasNodeKind.PROTOCOL,
    id,
    nodeData.protocolStepId,
  )
  const selfHighlight = useStore(canvasStore.store, (s): HighlightMode => {
    if (s.hoveredStepId === id) return HighlightMode.HOVER
    return HighlightMode.NONE
  })
  const baseHighlight = isAgent ? protocolHighlight : selfHighlight

  // --- Share mode (steps only) ---
  const shareActive = useStore(shareStore.store, isAgent ? () => false : shareStore.selectActive)
  const shareSourceId = useStore(shareStore.store, isAgent ? () => null : shareStore.selectSourceStepId)
  const pendingChatFocus = useStore(shareStore.store, isAgent ? () => null : shareStore.selectPendingChatFocus)
  const isShareSource = shareActive && shareSourceId === id

  useEffect(() => {
    if (pendingChatFocus === id) {
      queueMicrotask(() => { setActiveTabId('chat') })
      shareStore.clearPendingChatFocus()
    }
  }, [pendingChatFocus, id])

  useEffect(() => {
    if (isAgent && agentSourceStatus === 'running') {
      queueMicrotask(() => { setActiveTabId('stream') })
    } else if (!isAgent && stepExecStatus === 'running') {
      queueMicrotask(() => { setActiveTabId('live') })
    }
  }, [isAgent, agentSourceStatus, stepExecStatus])

  const shareOverlay = isShareSource ? <SharePickerPanel stepId={id} /> : undefined
  const effectiveHighlight = shareActive && !isShareSource ? HighlightMode.HOVER : baseHighlight

  // --- Build tabs ---
  const tabs = isAgent
    ? [
        { id: 'stream', icon: StreamOutlined, tooltip: 'Live Stream', content: <AgentStreamTab rosterAgentId={nodeData.rosterAgentId ?? ''} /> },
        { id: 'info', icon: InfoOutlined, tooltip: 'Info', content: <AgentInfoTab roleDescription={nodeData.roleDescription ?? ''} capabilities={nodeData.capabilities} /> },
      ]
    : buildStepTabs({
        stepId: id,
        archetype: nodeData.archetype,
        upstreamStepNames: nodeData.upstreamStepNames,
        documentDefs,
        documentActions,
        includeLiveStream: true,
      })

  // --- Header ---
  const subtitle = resolveSubtitle({
    archetype: nodeData.archetype,
    rosterNames: nodeData.rosterNames,
    documentNames: nodeData.documentNames,
    roomMemberNames: roomStepMembers.map((m) => m.name),
    parentStepName: nodeData.parentStepName,
  })

  const enterFocusMode = useEnterFocusMode()
  const handleEnterFocusMode = useCallback(() => { enterFocusMode(id) }, [enterFocusMode, id])

  // --- Handles (shared between MINIMAL and full render) ---
  const agentHandles = (
    <>
      <CanvasHandle type="target" position={Position.Bottom} id="agent-input" color={accentColor} variant="passive" />
      <CanvasHandle type="source" position={Position.Top} id="agent-output" color={accentColor} variant="passive" />
      <CanvasHandle type="source" position={Position.Right} id="agent-documents" color={DOCUMENT_NODE.ACCENT_COLOR} variant="passive" />
    </>
  )

  const stepExtraHandles = (
    <>
      {nodeData.archetype === Archetype.WORKFORCE && (
        <>
          <CanvasHandle type="source" position={Position.Top} id="documents" color={accentColor} />
          <CanvasHandle type="source" position={Position.Top} id="agents" color={accentColor} />
        </>
      )}
      <CanvasHandle type="source" position={Position.Bottom} id="notes" color={NOTES_ACCENT} variant="passive" />
    </>
  )

  // --- Minimal LOD ---
  if (detailLevel === DetailLevel.MINIMAL) {
    const highlight = getNodeHighlightStyles({
      selected: selected === true,
      accentColor,
      highlightMode: effectiveHighlight,
      themeMode: theme.palette.mode,
      variant: 'resizable',
    })
    return (
      <Box sx={{ width: '100%', height: '100%' }}>
        <MinimalNodeShell label={nodeData.label} accentColor={accentColor} borderColor={highlight.borderColor} boxShadow={highlight.boxShadow} />
        {isAgent ? agentHandles : (
          <>
            <CanvasHandle type="target" position={Position.Left} color={accentColor} />
            <CanvasHandle type="source" position={Position.Right} color={accentColor} />
            {stepExtraHandles}
          </>
        )}
      </Box>
    )
  }

  // --- Header badge ---
  const headerBadge = isExecuting ? (
    <ExecutionStatusBadge status={resolvedExecStatus} />
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
  ) : nodeData.archetype !== Archetype.BLANK ? (
    <ProtocolBadge color={config.color} label={config.label} animated />
  ) : undefined

  const headerActions = isAgent ? undefined : (
    <IconButton className="nodrag" onClick={handleEnterFocusMode} size="small" sx={{ flexShrink: 0, width: 28, height: 28, color: 'text.secondary', '&:hover': { color: 'text.primary' } }}>
      <OpenInFullOutlined sx={{ fontSize: 16 }} />
    </IconButton>
  )

  const IconComponent = config.icon
  const headerElement = (
    <NodeHeader
      icon={<IconComponent sx={{ fontSize: isAgent ? 18 : 20, color: config.color }} />}
      title={nodeData.label}
      subtitle={subtitle}
      accentColor={config.color}
      size={isAgent ? 'standard' : 'large'}
      badge={headerBadge}
      actions={headerActions}
    />
  )

  return (
    <>
      <CanvasFormNode
        nodeId={id}
        header={headerElement}
        headerHeight={isAgent ? 52 : undefined}
        tabs={tabs}
        activeTabId={activeTabId}
        onTabChange={setActiveTabId}
        selected={selected === true}
        accentColor={accentColor}
        highlightMode={effectiveHighlight}
        overlay={shareOverlay}
        {...(isAgent
          ? { handles: agentHandles, constraints: AGENT_CONSTRAINTS }
          : { extraHandles: stepExtraHandles }
        )}
      />
    </>
  )
}

const dynamicNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const DynamicNode = memo(DynamicNodeComponent, dynamicNodeEqual)

export { DynamicNode }
export type { DynamicNodeData }
