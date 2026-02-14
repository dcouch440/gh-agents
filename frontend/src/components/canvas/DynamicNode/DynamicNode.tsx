import { memo, useState, useCallback } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'

import AutoAwesomeOutlined from '@mui/icons-material/AutoAwesomeOutlined'
import InputOutlined from '@mui/icons-material/InputOutlined'
import DescriptionOutlined from '@mui/icons-material/DescriptionOutlined'
import GroupsOutlined from '@mui/icons-material/GroupsOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import BugReportOutlined from '@mui/icons-material/BugReportOutlined'
import { useStore, workflowStore, canvasStore } from '@/stores'
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
import { ChatTab } from './tabs/ChatTab'
import { InputsOutputsTab } from './tabs/InputsOutputsTab'
import { DocumentsTab } from './tabs/DocumentsTab'
import { AgentRosterTab } from './tabs/AgentRosterTab'
import { RoomMembersTab } from './tabs/RoomMembersTab'
import { DebugLogTab } from './tabs/DebugLogTab'

type DynamicNodeData = {
  kind: CanvasNodeKind
  archetype: ArchetypeType
  label: string
  description: string
  documentNames: string[]
  upstreamStepNames: string[]
  promptValue: string
  modelId: string | null
  agentName: string | null
}

function DynamicNodeComponent({ id, data, selected }: NodeProps) {
  const [activeTabId, setActiveTabId] = useState('chat')
  const [adding, setAdding] = useState(false)
  const nodeData = data as DynamicNodeData

  const config = ARCHETYPE_CONFIGS[nodeData.archetype]
  const accentColor = config.color

  const documentDefs = useStore(workflowStore.store, workflowStore.selectStepDocumentDefs(id))

  const selfHighlight = useStore(canvasStore.store, (s): HighlightMode => {
    if (s.hoveredStepId === id) return HighlightMode.HOVER
    return HighlightMode.NONE
  })

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

  // Debug tab always last
  tabs.push({
    id: 'debug',
    icon: BugReportOutlined,
    tooltip: 'Debug Log',
    content: <DebugLogTab stepId={id} />,
  })

  // Header subtitle
  const subtitle = nodeData.archetype === Archetype.DOCUMENTER && nodeData.documentNames.length > 0
    ? nodeData.documentNames.join(' \u00b7 ')
    : null

  return (
    <CanvasFormNode
      header={
        <DynamicNodeHeader
          name={nodeData.label}
          archetype={nodeData.archetype}
          subtitle={subtitle}
        />
      }
      tabs={tabs}
      activeTabId={activeTabId}
      onTabChange={setActiveTabId}
      selected={selected === true}
      accentColor={accentColor}
      highlightMode={selfHighlight}
      extraHandles={
        nodeData.archetype === Archetype.DOCUMENTER ? (
          <CanvasHandle type="source" position={Position.Top} id="documents" color={accentColor} />
        ) : undefined
      }
    />
  )
}

const dynamicNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const DynamicNode = memo(DynamicNodeComponent, dynamicNodeEqual)

export { DynamicNode }
export type { DynamicNodeData }
