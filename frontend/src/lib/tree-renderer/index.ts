// Components
export { TreeCanvas, TreeNodeBox, TreeNodeLabel, TreeEdgePath, StatusIndicator, TreeDefs, TreeNodeGroup } from './components'
export type { TreeNodeBoxProps, TreeNodeLabelProps, TreeEdgePathProps, StatusIndicatorProps, TreeNodeGroupProps } from './components'

// Layout
export { computeLayout } from './layout'

// Theme
export { DEFAULT_THEME, getStatusColor, themeToCSS } from './theme'

// Hooks
export { useNodeTransitions } from './hooks/useNodeTransitions'
export { usePanZoom } from './hooks/usePanZoom'

// Adapters
export { pipelineToTree } from './adapters/pipelineAdapter'
export { tasksToTree } from './adapters/taskAdapter'
export { agentHierarchyToTree } from './adapters/agentAdapter'
export type { PipelineMeta } from './adapters/pipelineAdapter'
export type { TaskMeta } from './adapters/taskAdapter'
export type { AgentMeta } from './adapters/agentAdapter'

// Types
export type {
  NodeStatus,
  Orientation,
  EdgeStyle,
  EdgeVariant,
  TreeNode,
  TreeEdgeData,
  TreeData,
  LayoutOptions,
  PositionedNode,
  PositionedEdge,
  LayoutResult,
  TreeCanvasProps,
  TreeTheme,
} from './types'
