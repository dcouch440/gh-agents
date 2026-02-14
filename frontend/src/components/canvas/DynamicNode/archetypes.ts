import type { SvgIconComponent } from '@mui/icons-material'
import DescriptionOutlined from '@mui/icons-material/DescriptionOutlined'
import GroupsOutlined from '@mui/icons-material/GroupsOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import RadioButtonUncheckedOutlined from '@mui/icons-material/RadioButtonUncheckedOutlined'
import type { ProtocolStepInfo } from '../mappers'

const Archetype = {
  DOCUMENTER: 'documenter',
  TASK_FORCE: 'task_force',
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
  [Archetype.DOCUMENTER]: {
    label: 'Documenter',
    color: '#D4793E',
    executionMode: 'documenter',
    icon: DescriptionOutlined,
    archetypeTabLabel: 'Documents',
    chatEmptyMessage: 'Ask me to help set up documents for this step.',
  },
  [Archetype.TASK_FORCE]: {
    label: 'Task Force',
    color: '#3b82f6',
    executionMode: 'task_force',
    icon: GroupsOutlined,
    archetypeTabLabel: 'Agent Roster',
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
  protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>,
  stepId?: string,
): Archetype => {
  if (step.execution_mode === 'documenter') return Archetype.DOCUMENTER
  if (stepId && protocolsByStep.get(stepId)?.protocol_type === 'documenter') return Archetype.DOCUMENTER
  if (step.execution_mode === 'room') return Archetype.ROOM
  if (step.execution_mode === 'task_force') return Archetype.TASK_FORCE
  return Archetype.BLANK
}

export { Archetype, ARCHETYPE_CONFIGS, resolveArchetype }
export type { ArchetypeConfig }
