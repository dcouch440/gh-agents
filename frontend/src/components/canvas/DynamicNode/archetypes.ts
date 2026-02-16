import type { SvgIconComponent } from '@mui/icons-material'
import GroupsOutlined from '@mui/icons-material/GroupsOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import RadioButtonUncheckedOutlined from '@mui/icons-material/RadioButtonUncheckedOutlined'
import type { ProtocolStepInfo } from '../mappers'

const Archetype = {
  WORKFORCE: 'workforce',
  ROOM: 'room',
  BLANK: 'blank',
} as const

type Archetype = (typeof Archetype)[keyof typeof Archetype]

type ArchetypeConfig = {
  label: string
  color: string
  executionMode: string
  icon: SvgIconComponent
  archetypeTabLabel: string
  chatEmptyMessage: string
}

const ARCHETYPE_CONFIGS: Record<Archetype, ArchetypeConfig> = {
  [Archetype.WORKFORCE]: {
    label: 'Workforce',
    color: '#3b82f6',
    executionMode: 'workforce',
    icon: GroupsOutlined,
    archetypeTabLabel: 'Team',
    chatEmptyMessage: 'Describe your mission and I\'ll help build the team.',
  },
  [Archetype.ROOM]: {
    label: 'Room',
    color: '#a78bfa',
    executionMode: 'room',
    icon: ForumOutlined,
    archetypeTabLabel: 'Members',
    chatEmptyMessage: 'Tell me about the meeting you want to set up.',
  },
  [Archetype.BLANK]: {
    label: 'Blank',
    color: '#7d8590',
    executionMode: 'single',
    icon: RadioButtonUncheckedOutlined,
    archetypeTabLabel: '',
    chatEmptyMessage: 'Tell me what you\'d like this node to do.',
  },
}

const resolveArchetype = (
  step: { execution_mode: string },
  _protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>,
  _stepId?: string,
): Archetype => {
  if (step.execution_mode === 'workforce') return Archetype.WORKFORCE
  if (step.execution_mode === 'room') return Archetype.ROOM
  return Archetype.BLANK
}

export { Archetype, ARCHETYPE_CONFIGS, resolveArchetype }
export type { ArchetypeConfig }
