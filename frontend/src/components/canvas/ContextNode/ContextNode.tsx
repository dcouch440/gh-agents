import { memo, useState, useCallback } from 'react'
import { Position, NodeResizer } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { workflowStore } from '@/stores'
import { CanvasHandle } from '../CanvasHandle'
import { GREYSCALE_ACCENT } from '../constants'
import { CONTEXT_NODE } from './constants'
import { ContextNodeHeader } from './ContextNodeHeader'
import { ContextNodeContent } from './ContextNodeContent'
import type { ContextNodeData } from './types'
import { nodeDataEqual } from '../mappers'
import { useProtocolHighlight, CanvasNodeKind, HighlightMode } from '../useProtocolHighlight'

function ContextNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const nodeData = data as ContextNodeData
  const highlightMode = useProtocolHighlight(CanvasNodeKind.CONTEXT, id, nodeData.protocolStepId)
  const hasProtocol = nodeData.protocolColor !== null
  const accentColor = nodeData.protocolColor ?? GREYSCALE_ACCENT
  const [hovered, setHovered] = useState(false)

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
        border: hasProtocol ? 3 : 2,
        borderStyle: hasProtocol ? 'dashed' : 'solid',
        borderColor: selected
          ? accentColor
          : hasProtocol
            ? highlightMode === HighlightMode.SELECT
              ? accentColor
              : highlightMode === HighlightMode.HOVER
                ? `${accentColor}80`
                : `${accentColor}50`
            : 'divider',
        boxShadow: selected
          ? theme.palette.mode === 'dark'
            ? `0 0 0 1px ${accentColor}40, 0 8px 32px ${accentColor}22, 0 2px 8px rgba(0, 0, 0, 0.3)`
            : `0 0 0 1px ${accentColor}30, 0 12px 40px rgba(45, 27, 14, 0.18), 0 4px 12px ${accentColor}18`
          : highlightMode === HighlightMode.SELECT
            ? `0 0 0 1px ${accentColor}40, 0 8px 32px ${accentColor}22`
            : highlightMode === HighlightMode.HOVER
              ? `0 0 0 1px ${accentColor}20, 0 6px 24px ${accentColor}14`
              : theme.palette.mode === 'dark'
                ? '0 8px 32px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.3)'
                : '0 8px 32px rgba(45, 27, 14, 0.14), 0 2px 8px rgba(45, 27, 14, 0.08)',
        transition: 'border-color 150ms ease, box-shadow 150ms ease, border-style 150ms ease',
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
        <ContextNodeContent content={nodeData.content} accentColor={accentColor} onChange={handleContentChange} />
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
