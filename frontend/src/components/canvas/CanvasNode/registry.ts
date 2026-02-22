import type { SvgIconComponent } from '@mui/icons-material'
import AccountTreeOutlined from '@mui/icons-material/AccountTreeOutlined'
import GroupsOutlined from '@mui/icons-material/GroupsOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import RadioButtonUncheckedOutlined from '@mui/icons-material/RadioButtonUncheckedOutlined'
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import RepeatOutlined from '@mui/icons-material/RepeatOutlined'
import SettingsOutlined from '@mui/icons-material/SettingsOutlined'
import type { ResizeSizeConstraints } from '../CanvasFormNode'
import type { ProtocolStepInfo } from '../mappers/types'
import { CanvasNodeKind } from '../canvasKinds'
import type { NodeVariant, LayoutMode } from './types'

// ---------------------------------------------------------------------------
// Archetype — backward-compat alias (tabbed variants only)
// ---------------------------------------------------------------------------

const Archetype = {
  WORKFORCE: 'workforce',
  MANAGER: 'manager',
  ROOM: 'room',
  BLANK: 'blank',
  AGENT: 'agent',
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
  [Archetype.MANAGER]: {
    label: 'Manager',
    color: '#8b5cf6',
    executionMode: 'manager',
    icon: AccountTreeOutlined,
    archetypeTabLabel: 'Team',
    chatEmptyMessage: 'Describe your goals and I\'ll coordinate the team.',
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
  [Archetype.AGENT]: {
    label: 'Agent',
    color: '#06b6d4',
    executionMode: 'agent',
    icon: SmartToyOutlined,
    archetypeTabLabel: 'Info',
    chatEmptyMessage: '',
  },
}

// ---------------------------------------------------------------------------
// Variant config
// ---------------------------------------------------------------------------

type VariantConfig = {
  label: string
  color: string
  icon: SvgIconComponent
  layout: LayoutMode
  canvasNodeKind: CanvasNodeKind
  defaultWidth: number
  defaultHeight: number
  constraints: ResizeSizeConstraints | null
}

const VARIANT_CONFIGS: Record<NodeVariant, VariantConfig> = {
  workforce: {
    label: 'Workforce',
    color: '#3b82f6',
    icon: GroupsOutlined,
    layout: 'tabbed',
    canvasNodeKind: CanvasNodeKind.PROTOCOL,
    defaultWidth: 560,
    defaultHeight: 500,
    constraints: { minWidth: 360, minHeight: 300, maxWidth: 1800, maxHeight: 1600 },

  },
  manager: {
    label: 'Manager',
    color: '#8b5cf6',
    icon: AccountTreeOutlined,
    layout: 'tabbed',
    canvasNodeKind: CanvasNodeKind.PROTOCOL,
    defaultWidth: 560,
    defaultHeight: 500,
    constraints: { minWidth: 360, minHeight: 300, maxWidth: 1800, maxHeight: 1600 },

  },
  room: {
    label: 'Room',
    color: '#a78bfa',
    icon: ForumOutlined,
    layout: 'tabbed',
    canvasNodeKind: CanvasNodeKind.PROTOCOL,
    defaultWidth: 560,
    defaultHeight: 500,
    constraints: { minWidth: 360, minHeight: 300, maxWidth: 1800, maxHeight: 1600 },

  },
  blank: {
    label: 'Blank',
    color: '#7d8590',
    icon: RadioButtonUncheckedOutlined,
    layout: 'tabbed',
    canvasNodeKind: CanvasNodeKind.PROTOCOL,
    defaultWidth: 560,
    defaultHeight: 500,
    constraints: { minWidth: 360, minHeight: 300, maxWidth: 1800, maxHeight: 1600 },

  },
  agent: {
    label: 'Agent',
    color: '#06b6d4',
    icon: SmartToyOutlined,
    layout: 'tabbed',
    canvasNodeKind: CanvasNodeKind.AGENT,
    defaultWidth: 420,
    defaultHeight: 360,
    constraints: { minWidth: 360, minHeight: 300, maxWidth: 1200, maxHeight: 1000 },

  },
  context: {
    label: 'Context',
    color: '#10b981',
    icon: SettingsOutlined, // Placeholder — EditorLayout uses ContextNodeIcon directly
    layout: 'editor',
    canvasNodeKind: CanvasNodeKind.CONTEXT,
    defaultWidth: 560,
    defaultHeight: 500,
    constraints: { minWidth: 360, minHeight: 300, maxWidth: 1800, maxHeight: 1600 },

  },
  input: {
    label: 'Input',
    color: '#f59e0b',
    icon: SettingsOutlined, // Placeholder — EditorLayout uses InputNodeIcon directly
    layout: 'editor',
    canvasNodeKind: CanvasNodeKind.INPUT,
    defaultWidth: 560,
    defaultHeight: 500,
    constraints: { minWidth: 360, minHeight: 300, maxWidth: 1800, maxHeight: 1600 },

  },
  step: {
    label: 'Step',
    color: '#7d8590',
    icon: SettingsOutlined,
    layout: 'card',
    canvasNodeKind: CanvasNodeKind.STEP,
    defaultWidth: 260,
    defaultHeight: 0, // Auto-height
    constraints: null,

  },
  sub_workflow: {
    label: 'Sub-Workflow',
    color: '#10b981',
    icon: SmartToyOutlined,
    layout: 'compact',
    canvasNodeKind: CanvasNodeKind.SUB_WORKFLOW,
    defaultWidth: 180,
    defaultHeight: 56,
    constraints: null,

  },
}

// ---------------------------------------------------------------------------
// Sizing constants — backward-compat aliases
// ---------------------------------------------------------------------------

const AGENT_DEFAULTS = {
  DEFAULT_WIDTH: VARIANT_CONFIGS.agent.defaultWidth,
  DEFAULT_HEIGHT: VARIANT_CONFIGS.agent.defaultHeight,
} as const

const AGENT_CONSTRAINTS: ResizeSizeConstraints = VARIANT_CONFIGS.agent.constraints!

// ---------------------------------------------------------------------------
// Variant resolution
// ---------------------------------------------------------------------------

const resolveArchetype = (
  step: { execution_mode: string },
  _protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>,
  _stepId?: string,
): Archetype => {
  if (step.execution_mode === 'workforce') return Archetype.WORKFORCE
  if (step.execution_mode === 'manager') return Archetype.MANAGER
  if (step.execution_mode === 'room') return Archetype.ROOM
  return Archetype.BLANK
}

const resolveVariant = (
  step: { execution_mode: string },
  _protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>,
  _stepId?: string,
): NodeVariant => {
  if (step.execution_mode === 'context') return 'context'
  if (step.execution_mode === 'input') return 'input'
  if (step.execution_mode === 'sub_workflow') return 'sub_workflow'
  if (step.execution_mode === 'workforce') return 'workforce'
  if (step.execution_mode === 'manager') return 'manager'
  if (step.execution_mode === 'room') return 'room'
  // Could resolve to 'blank' for known tabbed types in the future
  return 'step'
}

// ---------------------------------------------------------------------------
// Step type icons (card layout)
// ---------------------------------------------------------------------------

const STEP_TYPE_ICONS: Record<string, SvgIconComponent> = {
  single: SmartToyOutlined,
  for_each: RepeatOutlined,
  room: ForumOutlined,
}

const DEFAULT_STEP_TYPE_ICON: SvgIconComponent = SettingsOutlined

export {
  Archetype,
  ARCHETYPE_CONFIGS,
  AGENT_DEFAULTS,
  AGENT_CONSTRAINTS,
  VARIANT_CONFIGS,
  STEP_TYPE_ICONS,
  DEFAULT_STEP_TYPE_ICON,
  resolveArchetype,
  resolveVariant,
}
export type { ArchetypeConfig, VariantConfig }
