// Component
export { CanvasNode } from './CanvasNode'

// Layouts
export { TabbedLayout, EditorLayout, CardLayout, CompactLayout } from './layouts'

// Types
export type {
  CanvasNodeData,
  TabbedNodeData,
  AgentNodeData,
  EditorNodeData,
  CardNodeData,
  CompactNodeData,
  LayoutMode,
} from './types'
export { NodeVariant } from './types'

// Registry
export {
  Archetype,
  ARCHETYPE_CONFIGS,
  AGENT_DEFAULTS,
  AGENT_CONSTRAINTS,
  VARIANT_CONFIGS,
  resolveArchetype,
  resolveVariant,
} from './registry'
export type { ArchetypeConfig, VariantConfig } from './registry'

// Shell
export { TabStrip } from './shell'
export type { TabStripProps, TabStripVariant } from './shell'

// Hooks
export { useDynamicNodeExecution } from './hooks'
export { useStepStoreData } from './hooks'

// Tabs
export { buildStepTabs } from './tabs/buildStepTabs'

// Resolve subtitle
export { resolveSubtitle } from './resolveSubtitle'
