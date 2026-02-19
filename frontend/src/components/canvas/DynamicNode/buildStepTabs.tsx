import AutoAwesomeOutlined from '@mui/icons-material/AutoAwesomeOutlined'
import GroupsOutlined from '@mui/icons-material/GroupsOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import BugReportOutlined from '@mui/icons-material/BugReportOutlined'
import AssignmentOutlined from '@mui/icons-material/AssignmentOutlined'
import StickyNote2Outlined from '@mui/icons-material/StickyNote2Outlined'
import type { CanvasFormTab } from '../CanvasFormNode'
import type { Archetype as ArchetypeType } from './archetypes'
import { Archetype } from './archetypes'
import { ChatTab } from './tabs/ChatTab'
import { LiveStreamTab } from './tabs/LiveStreamTab'
import { AgentRosterTab } from './tabs/AgentRosterTab'
import { RoomMembersTab } from './tabs/RoomMembersTab'
import { DebugLogTab } from './tabs/DebugLogTab'
import { NotesTab } from './tabs/NotesTab'

type BuildStepTabsParams = {
  stepId: string
  archetype: ArchetypeType
  includeLiveStream?: boolean
  focusMode?: boolean
}

const buildStepTabs = ({
  stepId,
  archetype,
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
      icon: AssignmentOutlined,
      tooltip: 'Run Results',
      content: <LiveStreamTab stepId={stepId} />,
    })
  }

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
    id: 'debug',
    icon: BugReportOutlined,
    tooltip: 'Debug Log',
    content: <DebugLogTab stepId={stepId} />,
  })

  return tabs
}

export { buildStepTabs }
export type { BuildStepTabsParams }
