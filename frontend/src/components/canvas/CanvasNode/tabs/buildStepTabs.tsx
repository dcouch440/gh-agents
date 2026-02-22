import AutoAwesomeOutlined from '@mui/icons-material/AutoAwesomeOutlined'
import GroupsOutlined from '@mui/icons-material/GroupsOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import BugReportOutlined from '@mui/icons-material/BugReportOutlined'
import AssignmentOutlined from '@mui/icons-material/AssignmentOutlined'
import StickyNote2Outlined from '@mui/icons-material/StickyNote2Outlined'
import type { CanvasFormTab } from '../../CanvasFormNode'
import type { Archetype as ArchetypeType } from '../registry'
import { Archetype } from '../registry'
import { ChatTab } from './ChatTab'
import { ChatClearButton } from './ChatClearButton'
import { LiveStreamTab } from './LiveStreamTab'
import { AgentRosterTab } from './AgentRosterTab'
import { RoomMembersTab } from './RoomMembersTab'
import { DebugLogTab } from './DebugLogTab'
import { PlanTab } from './PlanTab'

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
      actions: <ChatClearButton stepId={stepId} />,
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

  // Manager sits above the DAG — no plan tab (plan is a node builder concept)
  if (archetype !== Archetype.MANAGER) {
    tabs.push({
      id: 'plan',
      icon: StickyNote2Outlined,
      tooltip: 'Plan',
      content: <PlanTab stepId={stepId} />,
    })
  }

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
