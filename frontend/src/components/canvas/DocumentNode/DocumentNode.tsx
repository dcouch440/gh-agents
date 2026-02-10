import { memo, useState } from 'react'
import { Handle, Position, NodeResizer } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { DOCUMENT_NODE } from './constants'
import { DocumentNodeHeader } from './DocumentNodeHeader'
import { DocumentNodeContent } from './DocumentNodeContent'
import type { DocumentNodeData } from './types'
import { nodeDataEqual } from '../mappers'

function DocumentNodeComponent({ data, selected }: NodeProps) {
  const theme = useTheme()
  const nodeData = data as DocumentNodeData
  const accentColor = DOCUMENT_NODE.ACCENT_COLOR
  const [hovered, setHovered] = useState(false)

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
        borderStyle: 'dashed',
        boxShadow: selected
          ? theme.palette.mode === 'dark'
            ? `0 0 0 1px ${accentColor}40, 0 8px 32px rgba(212, 121, 62, 0.14), 0 2px 8px rgba(0, 0, 0, 0.3)`
            : `0 0 0 1px ${accentColor}30, 0 12px 40px rgba(45, 27, 14, 0.18), 0 4px 12px rgba(212, 121, 62, 0.10)`
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
      <Handle
        type="target"
        position={Position.Left}
        style={{
          width: 8,
          height: 8,
          background: accentColor,
          border: `2px solid ${theme.palette.custom.bgHeader}`,
          pointerEvents: 'none',
        }}
      />

      <NodeResizer
        isVisible={hovered || selected === true}
        minWidth={DOCUMENT_NODE.MIN_WIDTH}
        minHeight={DOCUMENT_NODE.MIN_HEIGHT}
        maxWidth={DOCUMENT_NODE.MAX_WIDTH}
        maxHeight={DOCUMENT_NODE.MAX_HEIGHT}
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
          height: DOCUMENT_NODE.HEADER_HEIGHT,
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
        <DocumentNodeHeader name={nodeData.label} documenterName={nodeData.documenterName} accentColor={accentColor} />
      </Box>

      {/* Content area — read-only */}
      <Box className="nowheel nodrag nopan" sx={{ flex: 1, overflow: 'hidden', position: 'relative' }}>
        <DocumentNodeContent content={nodeData.content} accentColor={accentColor} />
      </Box>
    </Box>
  )
}

const documentNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const DocumentNode = memo(DocumentNodeComponent, documentNodeEqual)

export { DocumentNode }
