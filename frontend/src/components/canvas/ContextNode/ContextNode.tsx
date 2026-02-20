import { memo, useCallback } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore, shareStore } from '@/stores'
import { SharePickerPanel } from '../SharePickerPanel'
import { CanvasHandle } from '../CanvasHandle'
import { STEP_TYPE_COLORS, GREYSCALE_ACCENT, DetailLevel } from '../constants'
import { CONTEXT_NODE } from './constants'
import { ContextNodeIcon } from '../Icons/ContextNodeIcon'
import { ProtocolBadge } from '../ProtocolBadge'
import { NodeHeader } from '../execution'
import { ContextNodeContent } from './ContextNodeContent'
import type { ContextNodeData } from './types'
import { nodeDataEqual } from '../mappers'
import { CanvasNodeKind } from '../canvasKinds'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'
import { useCanvasLOD } from '../useCanvasLOD'
import { MinimalNodeShell } from '../MinimalNodeShell'
import { ResizableNodeShell, toConstraints } from '../ResizableNodeShell'

function ContextNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const detailLevel = useCanvasLOD()
  const nodeData = data as ContextNodeData
  const highlightMode = useProtocolHighlight(CanvasNodeKind.CONTEXT, id, nodeData.protocolStepId)
  const rawAccent = STEP_TYPE_COLORS['context'] ?? GREYSCALE_ACCENT
  const accentColor = theme.palette.mode === 'light' ? theme.palette.custom.accent : rawAccent
  const isShareSource = useStore(shareStore.store, (s) => s.active && s.sourceStepId === id)
  const highlight = getNodeHighlightStyles({
    selected: selected === true,
    accentColor,
    highlightMode,
    themeMode: theme.palette.mode,
    variant: 'resizable',
  })

  const handleContentChange = useCallback(
    (value: string) => {
      workflowStore.patchStepLocal(id, { prompt_template: value })
    },
    [id],
  )

  if (detailLevel === DetailLevel.MINIMAL) {
    return (
      <Box sx={{ width: '100%', height: '100%' }}>
        <MinimalNodeShell
          label={nodeData.label}
          accentColor={accentColor}
          borderColor={highlight.borderColor}
          boxShadow={highlight.boxShadow}
        />
        <CanvasHandle type="source" position={Position.Bottom} color={accentColor} />
      </Box>
    )
  }

  return (
    <ResizableNodeShell
      nodeId={id}
      selected={selected === true}
      accentColor={accentColor}
      highlight={highlight}
      constraints={toConstraints(CONTEXT_NODE)}
      handles={<CanvasHandle type="source" position={Position.Bottom} color={accentColor} />}
    >
      {/* Header — draggable area */}
      <Box
        sx={{
          height: CONTEXT_NODE.HEADER_HEIGHT,
          overflow: 'hidden',
          borderBottom: 1,
          borderColor: 'divider',
          display: 'flex',
          alignItems: 'center',
          backgroundColor: theme.palette.custom.screenBg,
          flexShrink: 0,
          cursor: 'grab',
          '&:active': { cursor: 'grabbing' },
        }}
      >
        <NodeHeader
          icon={<ContextNodeIcon color={accentColor} size={18} />}
          title={nodeData.label}
          subtitle="Injected directly in every agent"
          accentColor={accentColor}
          badge={<ProtocolBadge color={accentColor} label="Context" animated />}
        />
      </Box>

      {/* Content area — interactive, no drag */}
      <Box className="nowheel nodrag nopan" sx={{ flex: 1, minHeight: 0, overflow: 'hidden', position: 'relative' }}>
        {isShareSource ? (
          <SharePickerPanel stepId={id} />
        ) : (
          <ContextNodeContent content={nodeData.content} accentColor={accentColor} onChange={handleContentChange} />
        )}
      </Box>
    </ResizableNodeShell>
  )
}

const contextNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const ContextNode = memo(ContextNodeComponent, contextNodeEqual)

export { ContextNode }
