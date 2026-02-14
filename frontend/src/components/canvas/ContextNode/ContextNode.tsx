import { memo, useState, useCallback } from 'react'
import { Position, NodeResizer } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore, shareStore } from '@/stores'
import { SharePickerPanel } from '../SharePickerPanel'
import { CanvasHandle } from '../CanvasHandle'
import { STEP_TYPE_COLORS, GREYSCALE_ACCENT } from '../constants'
import { CONTEXT_NODE } from './constants'
import { ContextNodeHeader } from './ContextNodeHeader'
import { ContextNodeContent } from './ContextNodeContent'
import type { ContextNodeData } from './types'
import { nodeDataEqual } from '../mappers'
import { CanvasNodeKind } from '../canvasKinds'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'

function ContextNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const nodeData = data as ContextNodeData
  const highlightMode = useProtocolHighlight(CanvasNodeKind.CONTEXT, id, nodeData.protocolStepId)
  const accentColor = STEP_TYPE_COLORS['context'] ?? GREYSCALE_ACCENT
  const isShareSource = useStore(shareStore.store, (s) => s.active && s.sourceStepId === id)
  const [hovered, setHovered] = useState(false)
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

  return (
    <Box
      onMouseEnter={() => {
        setHovered(true)
      }}
      onMouseLeave={() => {
        setHovered(false)
      }}
      sx={{
        width: '100%',
        height: '100%',
        borderRadius: '12px',
        backgroundColor: theme.palette.mode === 'light' ? theme.palette.custom.cavityBg : 'background.paper',
        border: 2,
        borderColor: highlight.borderColor,
        boxShadow: highlight.boxShadow,
        transition: 'border-color 150ms ease, box-shadow 150ms ease',
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'column',
        cursor: 'default',
      }}
    >
      <NodeResizer
        isVisible={hovered || selected === true}
        minWidth={CONTEXT_NODE.MIN_WIDTH}
        minHeight={CONTEXT_NODE.MIN_HEIGHT}
        maxWidth={CONTEXT_NODE.MAX_WIDTH}
        maxHeight={CONTEXT_NODE.MAX_HEIGHT}
        lineStyle={{ borderColor: 'transparent', borderWidth: 0 }}
        handleStyle={{
          width: 8,
          height: 8,
          borderRadius: 2,
          backgroundColor: 'transparent',
          borderColor: 'transparent',
        }}
      />

      {/* Header — draggable area */}
      <Box
        sx={{
          height: CONTEXT_NODE.HEADER_HEIGHT,
          overflow: 'hidden',
          borderBottom: 1,
          borderColor: 'divider',
          display: 'flex',
          alignItems: 'center',
          backgroundColor: theme.palette.custom.bgHeader,
          flexShrink: 0,
          cursor: 'grab',
          '&:active': { cursor: 'grabbing' },
        }}
      >
        <ContextNodeHeader name={nodeData.label} accentColor={accentColor} />
      </Box>

      {/* Content area — interactive, no drag */}
      <Box className="nowheel nodrag nopan" sx={{ flex: 1, overflow: 'hidden', position: 'relative' }}>
        {isShareSource ? (
          <SharePickerPanel stepId={id} />
        ) : (
          <ContextNodeContent content={nodeData.content} accentColor={accentColor} onChange={handleContentChange} />
        )}
      </Box>

      {/* Source handle only — context nodes are source-only, no target handle */}
      <CanvasHandle type="source" position={Position.Bottom} color={accentColor} />
    </Box>
  )
}

const contextNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const ContextNode = memo(ContextNodeComponent, contextNodeEqual)

export { ContextNode }
