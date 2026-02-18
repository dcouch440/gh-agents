import AutoAwesomeOutlined from '@mui/icons-material/AutoAwesomeOutlined'
import InputOutlined from '@mui/icons-material/InputOutlined'
import GroupsOutlined from '@mui/icons-material/GroupsOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import BugReportOutlined from '@mui/icons-material/BugReportOutlined'
import HistoryOutlined from '@mui/icons-material/HistoryOutlined'
import StreamOutlined from '@mui/icons-material/StreamOutlined'
import StickyNote2Outlined from '@mui/icons-material/StickyNote2Outlined'
import type { CanvasFormTab } from '../CanvasFormNode'
import type { Archetype as ArchetypeType } from './archetypes'
import { Archetype } from './archetypes'
import { ChatTab } from './tabs/ChatTab'
import { LiveStreamTab } from './tabs/LiveStreamTab'
import { InputsOutputsTab } from './tabs/InputsOutputsTab'
import { AgentRosterTab } from './tabs/AgentRosterTab'
import { RoomMembersTab } from './tabs/RoomMembersTab'
import { DebugLogTab } from './tabs/DebugLogTab'
import { LastRunTab } from './tabs/LastRunTab'
import { NotesTab } from './tabs/NotesTab'

type BuildStepTabsParams = {
  stepId: string
  archetype: ArchetypeType
  upstreamStepNames: readonly string[]
  includeLiveStream?: boolean
  focusMode?: boolean
}

const buildStepTabs = ({
  stepId,
  archetype,
  upstreamStepNames,
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
  } else if (archetype === Archetype.ROOM) {
    tabs.push({
      id: 'members',
      icon: ForumOutlined,
      tooltip: 'Members',
      content: <RoomMembersTab stepId={stepId} />,
    })
  }

  tabs.push({
    id: 'notes',
    icon: StickyNote2Outlined,
    tooltip: 'Assistant Notes',
    content: <NotesTab stepId={stepId} />,
  })

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
