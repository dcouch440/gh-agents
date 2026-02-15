import { memo, useState, useCallback, useEffect } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'

import AutoAwesomeOutlined from '@mui/icons-material/AutoAwesomeOutlined'
import InputOutlined from '@mui/icons-material/InputOutlined'
import DescriptionOutlined from '@mui/icons-material/DescriptionOutlined'
import GroupsOutlined from '@mui/icons-material/GroupsOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import BugReportOutlined from '@mui/icons-material/BugReportOutlined'
import HistoryOutlined from '@mui/icons-material/HistoryOutlined'
import { useStore, workflowStore, canvasStore, shareStore } from '@/stores'
import type { CreateDocumentDefRequest } from '@/types/workflow'
import { CanvasFormNode } from '../CanvasFormNode'
import type { CanvasFormTab } from '../CanvasFormNode'
import { CanvasHandle } from '../CanvasHandle'
import { nodeDataEqual } from '../mappers'
import { HighlightMode } from '../canvasKinds'
import type { CanvasNodeKind } from '../canvasKinds'
import { Archetype, ARCHETYPE_CONFIGS } from './archetypes'
import type { Archetype as ArchetypeType } from './archetypes'
import { DynamicNodeHeader } from './DynamicNodeHeader'
import { NodeExpandedModal } from './NodeExpandedModal'
import { ChatTab } from './tabs/ChatTab'
import { InputsOutputsTab } from './tabs/InputsOutputsTab'
import { DocumentsTab } from './tabs/DocumentsTab'
import { AgentRosterTab } from './tabs/AgentRosterTab'
import { RoomMembersTab } from './tabs/RoomMembersTab'
import { DebugLogTab } from './tabs/DebugLogTab'
import { LastRunTab } from './tabs/LastRunTab'
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
  const [activeTabId, setActiveTabId] = useState('chat')
  const [adding, setAdding] = useState(false)
  const [expanded, setExpanded] = useState(false)
  const nodeData = data as DynamicNodeData

  const config = ARCHETYPE_CONFIGS[nodeData.archetype]
  const accentColor = config.color

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
      id: 'io',
      icon: InputOutlined,
      tooltip: 'Inputs / Outputs',
      content: <InputsOutputsTab upstreamStepNames={nodeData.upstreamStepNames} />,
    },
  ]

  // Archetype-specific tab
  if (nodeData.archetype === Archetype.DOCUMENTER) {
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
  } else if (nodeData.archetype === Archetype.TASK_FORCE) {
    tabs.push({
      id: 'agents',
      icon: GroupsOutlined,
      tooltip: 'Agent Roster',
      content: <AgentRosterTab stepId={id} />,
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
    if (nodeData.archetype === Archetype.DOCUMENTER && nodeData.documentNames.length > 0)
      return nodeData.documentNames.join(' \u00b7 ')
    if (nodeData.archetype === Archetype.TASK_FORCE && nodeData.rosterNames.length > 0)
      return nodeData.rosterNames.join(' \u00b7 ')
    if (nodeData.archetype === Archetype.ROOM && roomStepMembers.length > 0)
      return roomStepMembers.map((m) => m.name).join(' \u00b7 ')
    return null
  })()

  const handleResizeEnd = useCallback(
    (width: number, height: number) => {
      void workflowStore.updateStep(id, { width, height })
    },
    [id],
  )

  const handleExpand = useCallback(() => {
    setExpanded(true)
  }, [])

  const handleCollapse = useCallback(() => {
    setExpanded(false)
  }, [])

  const issueDescriptions = stepIssues.map((issue) => issue.description)

  const headerElement = (
    <DynamicNodeHeader
      name={nodeData.label}
      archetype={nodeData.archetype}
      subtitle={subtitle}
      issueCount={stepIssues.length}
      issueDescriptions={issueDescriptions}
      onExpand={handleExpand}
    />
  )

  return (
    <>
      <CanvasFormNode
        header={headerElement}
        tabs={tabs}
        activeTabId={activeTabId}
        onTabChange={setActiveTabId}
        selected={selected === true}
        accentColor={accentColor}
        highlightMode={effectiveHighlight}
        overlay={shareOverlay}
        onResizeEnd={handleResizeEnd}
        extraHandles={
          <>
            {nodeData.archetype === Archetype.DOCUMENTER && (
              <CanvasHandle type="source" position={Position.Top} id="documents" color={accentColor} />
            )}
            <CanvasHandle type="source" position={Position.Bottom} id="notes" color="#f85149" variant="passive" />
          </>
        }
      />
      <NodeExpandedModal
        open={expanded}
        onClose={handleCollapse}
        header={
          <DynamicNodeHeader
            name={nodeData.label}
            archetype={nodeData.archetype}
            subtitle={subtitle}
            issueCount={stepIssues.length}
            issueDescriptions={issueDescriptions}
          />
        }
        tabs={tabs}
        activeTabId={activeTabId}
        onTabChange={setActiveTabId}
        accentColor={accentColor}
      />
    </>
  )
}

const dynamicNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const DynamicNode = memo(DynamicNodeComponent, dynamicNodeEqual)

export { DynamicNode }
export type { DynamicNodeData }
