import { useState, useCallback } from 'react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import AutoAwesomeOutlined from '@mui/icons-material/AutoAwesomeOutlined'
import InputOutlined from '@mui/icons-material/InputOutlined'
import DescriptionOutlined from '@mui/icons-material/DescriptionOutlined'
import GroupsOutlined from '@mui/icons-material/GroupsOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import BugReportOutlined from '@mui/icons-material/BugReportOutlined'
import HistoryOutlined from '@mui/icons-material/HistoryOutlined'
import { useStore, workflowStore } from '@/stores'
import { FOCUS_MODE } from '@/constants'
import type { CanvasFormTab } from '@/components/canvas/CanvasFormNode'
import { Archetype, ARCHETYPE_CONFIGS } from '@/components/canvas/DynamicNode/archetypes'
import type { Archetype as ArchetypeType } from '@/components/canvas/DynamicNode/archetypes'
import { ChatTab } from '@/components/canvas/DynamicNode/tabs/ChatTab'
import { InputsOutputsTab } from '@/components/canvas/DynamicNode/tabs/InputsOutputsTab'
import { DocumentsTab } from '@/components/canvas/DynamicNode/tabs/DocumentsTab'
import { AgentRosterTab } from '@/components/canvas/DynamicNode/tabs/AgentRosterTab'
import { RoomMembersTab } from '@/components/canvas/DynamicNode/tabs/RoomMembersTab'
import { DebugLogTab } from '@/components/canvas/DynamicNode/tabs/DebugLogTab'
import { LastRunTab } from '@/components/canvas/DynamicNode/tabs/LastRunTab'
import type { CreateDocumentDefRequest } from '@/types/workflow'
import { FocusHeader } from './FocusHeader'
import { FocusTabStrip } from './FocusTabStrip'

type FocusNodeViewProps = {
  stepId: string
  archetype: ArchetypeType
  stepName: string
  upstreamStepNames: string[]
  activeTabId: string
  onTabChange: (tabId: string) => void
}

function FocusNodeView({
  stepId,
  archetype,
  stepName,
  upstreamStepNames,
  activeTabId,
  onTabChange,
}: FocusNodeViewProps) {
  const theme = useTheme()
  const config = ARCHETYPE_CONFIGS[archetype]
  const accentColor = config.color

  const documentDefs = useStore(workflowStore.store, workflowStore.selectStepDocumentDefs(stepId))
  const rosterAgents = useStore(workflowStore.store, workflowStore.selectStepRoster(stepId))
  const roomStepMembers = useStore(workflowStore.store, workflowStore.selectRoomStepMembers(stepId))
  const stepIssues = useStore(workflowStore.store, workflowStore.selectStepIssues(stepId))

  const [adding, setAdding] = useState(false)

  const handleAddDocument = useCallback(() => {
    setAdding(true)
  }, [])

  const handleSubmitNew = useCallback(
    (body: CreateDocumentDefRequest) => {
      void workflowStore.createDocumentDef(stepId, body)
      setAdding(false)
    },
    [stepId],
  )

  const handleCancelAdd = useCallback(() => {
    setAdding(false)
  }, [])

  const handleRemoveDocument = useCallback(
    (defId: string) => {
      void workflowStore.deleteDocumentDef(stepId, defId)
    },
    [stepId],
  )

  // Build tabs — mirrors DynamicNode.tsx tab construction
  const tabs: CanvasFormTab[] = [
    {
      id: 'chat',
      icon: AutoAwesomeOutlined,
      tooltip: 'Chat',
      content: <ChatTab stepId={stepId} archetype={archetype} focusMode />,
    },
    {
      id: 'io',
      icon: InputOutlined,
      tooltip: 'Inputs / Outputs',
      content: <InputsOutputsTab upstreamStepNames={upstreamStepNames} />,
    },
  ]

  if (archetype === Archetype.DOCUMENTER) {
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
  } else if (archetype === Archetype.TASK_FORCE) {
    tabs.push({
      id: 'agents',
      icon: GroupsOutlined,
      tooltip: 'Agent Roster',
      content: <AgentRosterTab stepId={stepId} />,
    })
  } else if (archetype === Archetype.ROOM) {
    tabs.push({
      id: 'members',
      icon: ForumOutlined,
      tooltip: 'Members',
      content: <RoomMembersTab stepId={stepId} />,
    })
  }

  tabs.push({
    id: 'lastrun',
    icon: HistoryOutlined,
    tooltip: 'Last Run',
    content: <LastRunTab stepId={stepId} />,
  })

  tabs.push({
    id: 'debug',
    icon: BugReportOutlined,
    tooltip: 'Debug Log',
    content: <DebugLogTab stepId={stepId} />,
  })

  const activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0]

  const subtitle = (() => {
    if (archetype === Archetype.DOCUMENTER && documentDefs.length > 0)
      return documentDefs.map((d) => d.name).join(' \u00b7 ')
    if (archetype === Archetype.TASK_FORCE && rosterAgents.length > 0)
      return rosterAgents.map((a) => a.name).join(' \u00b7 ')
    if (archetype === Archetype.ROOM && roomStepMembers.length > 0)
      return roomStepMembers.map((m) => m.name).join(' \u00b7 ')
    return null
  })()

  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        backgroundColor: theme.palette.background.default,
      }}
    >
      {/* Centered content column */}
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          height: '100%',
          maxWidth: FOCUS_MODE.CONTENT_MAX_WIDTH,
          width: '100%',
          mx: 'auto',
        }}
      >
        {/* Header */}
        <FocusHeader
          name={stepName}
          archetype={archetype}
          subtitle={subtitle}
          issueCount={stepIssues.length}
          issueDescriptions={stepIssues.map((i) => i.description)}
        />

        {/* Tab strip */}
        <FocusTabStrip
          tabs={tabs}
          activeTabId={activeTabId}
          onTabChange={onTabChange}
          accentColor={accentColor}
        />

        {/* Content */}
        <Box
          sx={{
            flex: 1,
            minHeight: 0,
            overflow: 'hidden',
            position: 'relative',
            cursor: 'text',
            userSelect: 'text',
          }}
        >
          {activeTab?.content}
        </Box>
      </Box>
    </Box>
  )
}

export { FocusNodeView }
export type { FocusNodeViewProps }
