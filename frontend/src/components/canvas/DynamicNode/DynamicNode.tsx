import { memo, useState, useCallback, useEffect } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'

import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import AutoAwesomeOutlined from '@mui/icons-material/AutoAwesomeOutlined'
import InputOutlined from '@mui/icons-material/InputOutlined'
import DescriptionOutlined from '@mui/icons-material/DescriptionOutlined'
import GroupsOutlined from '@mui/icons-material/GroupsOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import BugReportOutlined from '@mui/icons-material/BugReportOutlined'
import HistoryOutlined from '@mui/icons-material/HistoryOutlined'
import { useStore, workflowStore, canvasStore, shareStore, focusModeStore, workflowExecutionStore } from '@/stores'
import { topoSortStepIds } from '@/utils/topoSort'
import type { CreateDocumentDefRequest } from '@/types/workflow'
import { CanvasFormNode } from '../CanvasFormNode'
import type { CanvasFormTab } from '../CanvasFormNode'
import { CanvasHandle } from '../CanvasHandle'
import { DetailLevel } from '../constants'
import { nodeDataEqual } from '../mappers'
import { HighlightMode } from '../canvasKinds'
import type { CanvasNodeKind } from '../canvasKinds'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'
import { useCanvasLOD } from '../useCanvasLOD'
import { MinimalNodeShell } from '../MinimalNodeShell'
import { Archetype, ARCHETYPE_CONFIGS } from './archetypes'
import type { Archetype as ArchetypeType } from './archetypes'
import IconButton from '@mui/material/IconButton'
import Tooltip from '@mui/material/Tooltip'
import Typography from '@mui/material/Typography'
import OpenInFullOutlined from '@mui/icons-material/OpenInFullOutlined'
import { ProtocolBadge } from '../ProtocolBadge'
import { NodeHeader, ExecutionStatusBadge, toExecutionStatus } from '../execution'
import { ChatTab } from './tabs/ChatTab'
import { InputsOutputsTab } from './tabs/InputsOutputsTab'
import { DocumentsTab } from './tabs/DocumentsTab'
import { AgentRosterTab } from './tabs/AgentRosterTab'
import { RoomMembersTab } from './tabs/RoomMembersTab'
import { DebugLogTab } from './tabs/DebugLogTab'
import { LastRunTab } from './tabs/LastRunTab'
import { LiveStreamTab } from './tabs/LiveStreamTab'
import StreamOutlined from '@mui/icons-material/StreamOutlined'
import { SharePickerPanel } from '../SharePickerPanel'

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
}

function DynamicNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const [activeTabId, setActiveTabId] = useState('chat')
  const [adding, setAdding] = useState(false)
  const detailLevel = useCanvasLOD()
  const nodeData = data as DynamicNodeData

  const config = ARCHETYPE_CONFIGS[nodeData.archetype]
  const accentColor = config.color

  // Execution state
  const stepExec = useStore(workflowExecutionStore.store, workflowExecutionStore.selectStepState(id))
  const execStatus = toExecutionStatus(stepExec?.status)
  const isExecuting = execStatus !== 'idle'

  const documentDefs = useStore(workflowStore.store, workflowStore.selectStepDocumentDefs(id))
  const roomStepMembers = useStore(workflowStore.store, workflowStore.selectRoomStepMembers(id))
  const stepIssues = useStore(workflowStore.store, workflowStore.selectStepIssues(id))

  const selfHighlight = useStore(canvasStore.store, (s): HighlightMode => {
    if (s.hoveredStepId === id) return HighlightMode.HOVER
    return HighlightMode.NONE
  })

  // Share mode subscriptions
  const shareActive = useStore(shareStore.store, shareStore.selectActive)
  const shareSourceId = useStore(shareStore.store, shareStore.selectSourceStepId)
  const pendingChatFocus = useStore(shareStore.store, shareStore.selectPendingChatFocus)
  const isShareSource = shareActive && shareSourceId === id

  // Force tab switch when this node is a share target
  useEffect(() => {
    if (pendingChatFocus === id) {
      queueMicrotask(() => {
        setActiveTabId('chat')
      })
      shareStore.clearPendingChatFocus()
    }
  }, [pendingChatFocus, id])

  // Auto-switch to live tab when execution starts
  useEffect(() => {
    if (execStatus === 'running') {
      queueMicrotask(() => {
        setActiveTabId('live')
      })
    }
  }, [execStatus])

  // Overlay for share source, highlight for potential targets
  const shareOverlay = isShareSource ? <SharePickerPanel stepId={id} /> : undefined
  const effectiveHighlight = shareActive && !isShareSource ? HighlightMode.HOVER : selfHighlight

  const handleAddDocument = useCallback(() => {
    setAdding(true)
  }, [])

  const handleSubmitNew = useCallback(
    (body: CreateDocumentDefRequest) => {
      void workflowStore.createDocumentDef(id, body)
      setAdding(false)
    },
    [id],
  )

  const handleCancelAdd = useCallback(() => {
    setAdding(false)
  }, [])

  const handleRemoveDocument = useCallback(
    (defId: string) => {
      void workflowStore.deleteDocumentDef(id, defId)
    },
    [id],
  )

  // Build tabs
  const tabs: CanvasFormTab[] = [
    {
      id: 'chat',
      icon: AutoAwesomeOutlined,
      tooltip: 'Chat',
      content: <ChatTab stepId={id} archetype={nodeData.archetype} />,
    },
    {
      id: 'live',
      icon: StreamOutlined,
      tooltip: 'Live Stream',
      content: <LiveStreamTab stepId={id} />,
    },
    {
      id: 'io',
      icon: InputOutlined,
      tooltip: 'Inputs / Outputs',
      content: <InputsOutputsTab upstreamStepNames={nodeData.upstreamStepNames} />,
    },
  ]

  // Archetype-specific tab
  if (nodeData.archetype === Archetype.WORKFORCE) {
    tabs.push({
      id: 'agents',
      icon: GroupsOutlined,
      tooltip: 'Agent Roster',
      content: <AgentRosterTab stepId={id} />,
    })
    tabs.push({
      id: 'documents',
      icon: DescriptionOutlined,
      tooltip: 'Documents',
      content: (
        <DocumentsTab
          documents={documentDefs}
          adding={adding}
          onAdd={handleAddDocument}
          onSubmitNew={handleSubmitNew}
          onCancelAdd={handleCancelAdd}
          onRemove={handleRemoveDocument}
        />
      ),
    })
  } else if (nodeData.archetype === Archetype.ROOM) {
    tabs.push({
      id: 'members',
      icon: ForumOutlined,
      tooltip: 'Members',
      content: <RoomMembersTab stepId={id} />,
    })
  }

  // Last Run tab
  tabs.push({
    id: 'lastrun',
    icon: HistoryOutlined,
    tooltip: 'Last Run',
    content: <LastRunTab stepId={id} />,
  })

  // Debug tab always last
  tabs.push({
    id: 'debug',
    icon: BugReportOutlined,
    tooltip: 'Debug Log',
    content: <DebugLogTab stepId={id} />,
  })

  // Header subtitle
  const subtitle = (() => {
    if (nodeData.archetype === Archetype.WORKFORCE) {
      const parts = [...nodeData.rosterNames, ...nodeData.documentNames]
      return parts.length > 0 ? parts.join(' \u00b7 ') : null
    }
    if (nodeData.archetype === Archetype.ROOM && roomStepMembers.length > 0)
      return roomStepMembers.map((m) => m.name).join(' \u00b7 ')
    return null
  })()

  const handleEnterFocusMode = useCallback(() => {
    const allSteps = workflowStore.store.getState()
    const stepsArr = [...allSteps.steps.byId.values()]
    const edgesArr = [...allSteps.edges.byId.values()]
    const ordered = topoSortStepIds(stepsArr, edgesArr)
    if (ordered.length > 0) {
      focusModeStore.enter(ordered, id)
    }
  }, [id])

  const issueDescriptions = stepIssues.map((issue) => issue.description)

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
        <MinimalNodeShell
          label={nodeData.label}
          accentColor={accentColor}
          borderColor={highlight.borderColor}
          boxShadow={highlight.boxShadow}
        />
        <CanvasHandle type="target" position={Position.Left} color={accentColor} />
        <CanvasHandle type="source" position={Position.Right} color={accentColor} />
        {nodeData.archetype === Archetype.WORKFORCE && (
          <>
            <CanvasHandle type="source" position={Position.Top} id="documents" color={accentColor} />
            <CanvasHandle type="source" position={Position.Top} id="agents" color={accentColor} />
          </>
        )}
        <CanvasHandle type="source" position={Position.Bottom} id="notes" color="#f85149" variant="passive" />
      </Box>
    )
  }

  const hasIssues = stepIssues.length > 0
  const ISSUE_COLOR = '#f85149'

  const headerBadge = isExecuting ? (
    <ExecutionStatusBadge status={execStatus} />
  ) : hasIssues ? (
    <Tooltip
      title={
        <Box sx={{ py: 0.5 }}>
          {issueDescriptions.map((desc, i) => (
            <Typography key={i} sx={{ fontSize: 12, lineHeight: 1.4 }}>
              {desc}
            </Typography>
          ))}
        </Box>
      }
      arrow
      placement="top"
    >
      <span>
        <ProtocolBadge color={ISSUE_COLOR} label={`${stepIssues.length} Issue${stepIssues.length > 1 ? 's' : ''}`} animated />
      </span>
    </Tooltip>
  ) : nodeData.archetype !== Archetype.BLANK ? (
    <ProtocolBadge color={config.color} label={config.label} animated />
  ) : undefined

  const headerActions = (
    <IconButton
      className="nodrag"
      onClick={handleEnterFocusMode}
      size="small"
      sx={{
        flexShrink: 0,
        width: 28,
        height: 28,
        color: 'text.secondary',
        '&:hover': { color: 'text.primary' },
      }}
    >
      <OpenInFullOutlined sx={{ fontSize: 16 }} />
    </IconButton>
  )

  const IconComponent = config.icon
  const headerElement = (
    <NodeHeader
      icon={<IconComponent sx={{ fontSize: 20, color: config.color }} />}
      title={nodeData.label}
      subtitle={subtitle}
      accentColor={config.color}
      size="large"
      badge={headerBadge}
      actions={headerActions}
    />
  )

  return (
    <>
      <CanvasFormNode
        nodeId={id}
        header={headerElement}
        tabs={tabs}
        activeTabId={activeTabId}
        onTabChange={setActiveTabId}
        selected={selected === true}
        accentColor={accentColor}
        highlightMode={effectiveHighlight}
        overlay={shareOverlay}
        extraHandles={
          <>
            {nodeData.archetype === Archetype.WORKFORCE && (
              <>
                <CanvasHandle type="source" position={Position.Top} id="documents" color={accentColor} />
                <CanvasHandle type="source" position={Position.Top} id="agents" color={accentColor} />
              </>
            )}
            <CanvasHandle type="source" position={Position.Bottom} id="notes" color="#f85149" variant="passive" />
          </>
        }
      />
    </>
  )
}

const dynamicNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const DynamicNode = memo(DynamicNodeComponent, dynamicNodeEqual)

export { DynamicNode }
export type { DynamicNodeData }
