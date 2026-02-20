import { memo } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, canvasStore } from '@/stores'
import { CanvasHandle } from '../CanvasHandle'
import { CANVAS, DetailLevel } from '../constants'
import { HighlightMode } from '../canvasKinds'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { useCanvasLOD } from '../useCanvasLOD'
import { MinimalNodeShell } from '../MinimalNodeShell'
import { nodeDataEqual } from '../mappers'
import { VARIANT_CONFIGS } from './registry'
import type { CanvasNodeData, TabbedNodeData, AgentNodeData, EditorNodeData, CardNodeData, CompactNodeData } from './types'
import { TabbedLayout, EditorLayout, CardLayout, CompactLayout } from './layouts'

function CanvasNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const detailLevel = useCanvasLOD()
  const nodeData = data as CanvasNodeData
  const variantConfig = VARIANT_CONFIGS[nodeData.variant]
  const isAgent = nodeData.variant === 'agent'

  // Accent color — resolved per-theme, per-variant from the active theme's node palette
  const accentColor = theme.palette.nodePalette[nodeData.variant]

  // Highlight
  const protocolHighlight = useProtocolHighlight(
    variantConfig.canvasNodeKind,
    id,
    nodeData.protocolStepId,
  )
  const selfHighlight = useStore(canvasStore.store, (s): HighlightMode => {
    if (s.hoveredStepId === id) return HighlightMode.HOVER
    return HighlightMode.NONE
  })

  // Tabbed variants use self highlight for step-level, protocol highlight for agent
  // Editor/card/compact use protocol highlight
  const highlightMode = isAgent ? protocolHighlight
    : variantConfig.layout === 'tabbed' ? selfHighlight
    : protocolHighlight

  const highlight = getNodeHighlightStyles({
    selected: selected === true,
    accentColor,
    highlightMode,
    themeMode: theme.palette.mode,
    variant: variantConfig.highlightVariant,
    screenBorder: theme.palette.custom.screenBorder,
    accentRing: theme.palette.custom.accentRing,
  })

  // --- Minimal LOD ---
  if (detailLevel === DetailLevel.MINIMAL) {
    return (
      <Box sx={{ width: variantConfig.layout === 'card' ? CANVAS.NODE_WIDTH : '100%', height: '100%' }}>
        <MinimalNodeShell label={nodeData.label} accentColor={accentColor} borderColor={highlight.borderColor} boxShadow={highlight.boxShadow} />
        {renderMinimalHandles(nodeData, accentColor)}
      </Box>
    )
  }

  // --- Full render by layout ---
  switch (variantConfig.layout) {
    case 'tabbed':
      return (
        <TabbedLayout
          nodeId={id}
          data={nodeData as TabbedNodeData | AgentNodeData}
          selected={selected === true}
          accentColor={accentColor}
          highlightMode={highlightMode}
        />
      )
    case 'editor':
      return (
        <EditorLayout
          nodeId={id}
          data={nodeData as EditorNodeData}
          selected={selected === true}
          accentColor={accentColor}
          highlight={highlight}
        />
      )
    case 'card':
      return (
        <CardLayout
          nodeId={id}
          data={nodeData as CardNodeData}
          selected={selected === true}
          accentColor={accentColor}
          highlight={highlight}
        />
      )
    case 'compact':
      return (
        <CompactLayout
          data={nodeData as CompactNodeData}
          accentColor={accentColor}
          highlight={highlight}
        />
      )
  }
}

/** Render handles for minimal LOD — varies by variant. */
const renderMinimalHandles = (data: CanvasNodeData, accentColor: string) => {
  switch (data.variant) {
    case 'agent':
      return (
        <>
          <CanvasHandle type="target" position={Position.Bottom} id="agent-input" color={accentColor} variant="passive" />
          <CanvasHandle type="source" position={Position.Top} id="agent-output" color={accentColor} variant="passive" />
        </>
      )
    case 'workforce':
      return (
        <>
          <CanvasHandle type="target" position={Position.Left} color={accentColor} style={{ top: '33%' }} />
          <CanvasHandle type="source" position={Position.Right} color={accentColor} />
          <CanvasHandle type="source" position={Position.Top} id="agents" color={accentColor} variant="passive" />
        </>
      )
    case 'room':
    case 'blank':
      return (
        <>
          <CanvasHandle type="target" position={Position.Left} color={accentColor} />
          <CanvasHandle type="source" position={Position.Right} color={accentColor} />
        </>
      )
    case 'context':
    case 'input':
      return <CanvasHandle type="source" position={Position.Bottom} color={accentColor} />
    case 'step':
      return (
        <>
          <CanvasHandle type="target" position={Position.Left} color={accentColor} />
          <CanvasHandle type="source" position={Position.Right} color={accentColor} />
        </>
      )
    case 'sub_workflow':
      return (
        <>
          <CanvasHandle type="target" position={Position.Left} color={accentColor} variant="small" />
          <CanvasHandle type="source" position={Position.Right} color={accentColor} variant="small" />
        </>
      )
  }
}

const canvasNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const CanvasNode = memo(CanvasNodeComponent, canvasNodeEqual)

export { CanvasNode }
