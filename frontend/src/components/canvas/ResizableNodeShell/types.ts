import type { ReactNode } from 'react'

type ResizeSizeConstraints = {
  minWidth: number
  minHeight: number
  maxWidth: number
  maxHeight: number
}

type ResizableNodeShellProps = {
  /** Node ID — used for hover subscription (canvasStore) and resize persistence routing. */
  nodeId: string
  /** Whether the ReactFlow node is selected (from NodeProps.selected). */
  selected: boolean
  /** Accent color for resize handles. */
  accentColor: string
  /** Highlight-derived border and shadow styles. */
  highlight: { borderColor: string; boxShadow: string }
  /** Min/max size constraints for the NodeResizer. */
  constraints: ResizeSizeConstraints
  /** Content rendered inside the zoomed inner container. */
  children: ReactNode
  /** Handles/elements rendered outside the zoom container at true pixel scale. */
  handles?: ReactNode
}

/** Maps UPPER_CASE node constants to the camelCase shape expected by ResizeSizeConstraints. */
const toConstraints = (c: {
  MIN_WIDTH: number
  MIN_HEIGHT: number
  MAX_WIDTH: number
  MAX_HEIGHT: number
}): ResizeSizeConstraints => ({
  minWidth: c.MIN_WIDTH,
  minHeight: c.MIN_HEIGHT,
  maxWidth: c.MAX_WIDTH,
  maxHeight: c.MAX_HEIGHT,
})

export { toConstraints }
export type { ResizeSizeConstraints, ResizableNodeShellProps }
