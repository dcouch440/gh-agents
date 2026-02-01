import type { ReactNode } from 'react'

type NodeStatus = 'pending' | 'running' | 'completed' | 'failed' | 'waiting' | 'skipped'

type Orientation = 'vertical' | 'horizontal'

type EdgeStyle = 'step' | 'curve' | 'straight'

type EdgeVariant = 'normal' | 'dependency' | 'approval'

type TreeNode<M = Record<string, unknown>> = {
  id: string
  label: string
  status: NodeStatus
  children: string[]
  metadata: M
}

type TreeEdgeData = {
  sourceId: string
  targetId: string
  label: string | null
  variant: EdgeVariant
}

type TreeData<M = Record<string, unknown>> = {
  nodes: Record<string, TreeNode<M>>
  rootIds: string[]
  edges: TreeEdgeData[]
}

type LayoutOptions = {
  orientation: Orientation
  nodeWidth: number
  nodeHeight: number
  horizontalGap: number
  verticalGap: number
  edgeStyle: EdgeStyle
}

type PositionedNode = {
  id: string
  x: number
  y: number
  width: number
  height: number
}

type PositionedEdge = {
  sourceId: string
  targetId: string
  path: string
  variant: EdgeVariant
  label: string | null
}

type LayoutResult = {
  nodes: PositionedNode[]
  edges: PositionedEdge[]
  width: number
  height: number
}

type TreeCanvasProps<M = Record<string, unknown>> = {
  data: TreeData<M>
  orientation?: Orientation
  layoutOptions?: Partial<LayoutOptions>
  theme?: Partial<TreeTheme>
  renderNode?: (node: TreeNode<M>, position: PositionedNode) => ReactNode
  onNodeClick?: (nodeId: string) => void
  onNodeHover?: (nodeId: string | null) => void
  className?: string
}

type TreeTheme = {
  colorPending: string
  colorRunning: string
  colorCompleted: string
  colorFailed: string
  colorWaiting: string
  colorSkipped: string
  colorNodeBg: string
  colorNodeBorder: string
  colorEdge: string
  colorEdgeActive: string
  colorLabel: string
  colorLabelSecondary: string
  fontFamily: string
  fontSize: number
  nodeRadius: number
  glowEnabled: boolean
}

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
}
