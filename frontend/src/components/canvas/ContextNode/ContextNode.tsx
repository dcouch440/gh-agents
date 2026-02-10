import { memo, useState, useCallback } from 'react'
import { Handle, Position, NodeResizer } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { workflowStore } from '@/stores'
import { CANVAS } from '../constants'
import { CONTEXT_NODE } from './constants'
import { ContextNodeHeader } from './ContextNodeHeader'
import { ContextNodeContent } from './ContextNodeContent'
import type { ContextNodeData } from './types'
import { nodeDataEqual } from '../mappers'

function ContextNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const nodeData = data as ContextNodeData
  const accentColor = CONTEXT_NODE.ACCENT_COLOR
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
        border: 2,
        borderColor: selected ? accentColor : 'divider',
        boxShadow: selected
          ? theme.palette.mode === 'dark'
            ? `0 0 0 1px ${accentColor}40, 0 8px 32px rgba(16, 185, 129, 0.14), 0 2px 8px rgba(0, 0, 0, 0.3)`
            : `0 0 0 1px ${accentColor}30, 0 12px 40px rgba(45, 27, 14, 0.18), 0 4px 12px rgba(16, 185, 129, 0.10)`
          : theme.palette.mode === 'dark'
            ? '0 8px 32px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.3)'
            : '0 8px 32px rgba(45, 27, 14, 0.14), 0 2px 8px rgba(45, 27, 14, 0.08)',
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
        <ContextNodeContent content={nodeData.content} accentColor={accentColor} onChange={handleContentChange} />
      </Box>

      {/* Source handle only — context nodes are source-only, no target handle */}
      <Handle
        type="source"
        position={Position.Right}
        style={{
          width: CANVAS.HANDLE_SIZE,
          height: CANVAS.HANDLE_SIZE,
          background: accentColor,
          border: `2px solid ${theme.palette.custom.bgHeader}`,
        }}
      />
    </Box>
  )
}

const contextNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const ContextNode = memo(ContextNodeComponent, contextNodeEqual)

export { ContextNode }
