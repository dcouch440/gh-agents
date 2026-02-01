import { useMemo } from 'react'
import type { TreeCanvasProps, TreeTheme } from '../types'
import { computeLayout } from '../layout'
import { DEFAULT_THEME, themeToCSS } from '../theme'
import { useNodeTransitions } from '../hooks/useNodeTransitions'
import { usePanZoom } from '../hooks/usePanZoom'
import { TreeDefs } from './TreeDefs'
import { TreeEdgePath } from './TreeEdgePath'
import { TreeNodeGroup } from './TreeNodeGroup'
import { TreeNodeBox } from './TreeNodeBox'
import { TreeNodeLabel } from './TreeNodeLabel'
import { StatusIndicator } from './StatusIndicator'
import '../tree-renderer.css'

const PADDING = 32

function TreeCanvas<M = Record<string, unknown>>({
  data,
  orientation = 'vertical',
  layoutOptions,
  theme: themeOverrides,
  renderNode,
  onNodeClick,
  onNodeHover,
  className,
}: TreeCanvasProps<M>) {
  const theme: TreeTheme = useMemo(
    () => ({ ...DEFAULT_THEME, ...themeOverrides }),
    [themeOverrides],
  )

  const layout = useMemo(
    () => computeLayout(data, { orientation, ...layoutOptions }),
    [data, orientation, layoutOptions],
  )

  const nodeIds = useMemo(() => layout.nodes.map((n) => n.id), [layout.nodes])
  const transitions = useNodeTransitions(nodeIds)
  const { state: panZoom, handlers, svgRef } = usePanZoom()

  const viewWidth = layout.width + PADDING * 2
  const viewHeight = layout.height + PADDING * 2

  const cssVars = useMemo(() => themeToCSS(theme), [theme])

  return (
    <svg
      ref={svgRef}
      className={`tree-canvas${className !== undefined ? ` ${className}` : ''}`}
      viewBox={`${-PADDING - panZoom.panX} ${-PADDING - panZoom.panY} ${viewWidth / panZoom.zoom} ${viewHeight / panZoom.zoom}`}
      width="100%"
      height={viewHeight}
      style={cssVars as React.CSSProperties}
      {...handlers}
    >
      <TreeDefs theme={theme} />

      {/* Edges first (behind nodes) */}
      <g className="tree-edges">
        {layout.edges.map((edge) => {
          const sourceNode = data.nodes[edge.sourceId]
          const targetNode = data.nodes[edge.targetId]
          return (
            <TreeEdgePath
              key={`${edge.sourceId}-${edge.targetId}`}
              edge={edge}
              sourceStatus={sourceNode?.status ?? 'pending'}
              targetStatus={targetNode?.status ?? 'pending'}
            />
          )
        })}
      </g>

      {/* Nodes */}
      <g className="tree-nodes">
        {layout.nodes.map((pos) => {
          const node = data.nodes[pos.id]
          if (node === undefined) return null

          const transition = transitions.get(pos.id) ?? 'stable'

          return (
            <TreeNodeGroup
              key={pos.id}
              x={pos.x}
              y={pos.y}
              nodeId={pos.id}
              transition={transition}
              onClick={onNodeClick}
              onHover={onNodeHover}
            >
              {renderNode !== undefined ? (
                renderNode(node, pos)
              ) : (
                <>
                  <TreeNodeBox width={pos.width} height={pos.height} status={node.status} />
                  <TreeNodeLabel y={16}>{node.label}</TreeNodeLabel>
                  <StatusIndicator
                    status={node.status}
                    x={pos.width - 18}
                    y={6}
                    size={12}
                    theme={theme}
                  />
                </>
              )}
            </TreeNodeGroup>
          )
        })}
      </g>
    </svg>
  )
}

export { TreeCanvas }
