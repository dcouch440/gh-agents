import AutoAwesomeOutlined from '@mui/icons-material/AutoAwesomeOutlined'
import InputOutlined from '@mui/icons-material/InputOutlined'
import DescriptionOutlined from '@mui/icons-material/DescriptionOutlined'
import GroupsOutlined from '@mui/icons-material/GroupsOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import BugReportOutlined from '@mui/icons-material/BugReportOutlined'
import HistoryOutlined from '@mui/icons-material/HistoryOutlined'
import StreamOutlined from '@mui/icons-material/StreamOutlined'
import type { CanvasFormTab } from '../CanvasFormNode'
import type { DocumentDef } from '@/types/workflow'
import type { Archetype as ArchetypeType } from './archetypes'
import { Archetype } from './archetypes'
import type { DocumentActions } from './useDocumentActions'
import { ChatTab } from './tabs/ChatTab'
import { LiveStreamTab } from './tabs/LiveStreamTab'
import { InputsOutputsTab } from './tabs/InputsOutputsTab'
import { DocumentsTab } from './tabs/DocumentsTab'
import { AgentRosterTab } from './tabs/AgentRosterTab'
import { RoomMembersTab } from './tabs/RoomMembersTab'
import { DebugLogTab } from './tabs/DebugLogTab'
import { LastRunTab } from './tabs/LastRunTab'

type BuildStepTabsParams = {
  stepId: string
  archetype: ArchetypeType
  upstreamStepNames: readonly string[]
  documentDefs: DocumentDef[]
  documentActions: DocumentActions
  includeLiveStream?: boolean
  focusMode?: boolean
}

const buildStepTabs = ({
  stepId,
  archetype,
  upstreamStepNames,
  documentDefs,
  documentActions,
  includeLiveStream = false,
  focusMode = false,
}: BuildStepTabsParams): CanvasFormTab[] => {
  const tabs: CanvasFormTab[] = [
    {
      id: 'chat',
      icon: AutoAwesomeOutlined,
      tooltip: 'Chat',
      content: <ChatTab stepId={stepId} archetype={archetype} focusMode={focusMode} />,
    },
  ]

  if (includeLiveStream) {
    tabs.push({
      id: 'live',
      icon: StreamOutlined,
      tooltip: 'Live Stream',
      content: <LiveStreamTab stepId={stepId} />,
    })
  }

  tabs.push({
    id: 'io',
    icon: InputOutlined,
    tooltip: 'Inputs / Outputs',
    content: <InputsOutputsTab upstreamStepNames={upstreamStepNames} />,
  })

  if (archetype === Archetype.WORKFORCE) {
    tabs.push({
      id: 'agents',
      icon: GroupsOutlined,
      tooltip: 'Agent Roster',
      content: <AgentRosterTab stepId={stepId} />,
    })
    tabs.push({
      id: 'documents',
      icon: DescriptionOutlined,
      tooltip: 'Documents',
      content: (
        <DocumentsTab
          documents={documentDefs}
          adding={documentActions.adding}
          onAdd={documentActions.onAdd}
          onSubmitNew={documentActions.onSubmitNew}
          onCancelAdd={documentActions.onCancelAdd}
          onRemove={documentActions.onRemove}
        />
      ),
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

  return tabs
}

export { buildStepTabs }
export type { BuildStepTabsParams }
