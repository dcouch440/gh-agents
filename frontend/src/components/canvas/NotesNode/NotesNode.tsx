import { memo, useState } from 'react'
import { Position, NodeResizer } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { CanvasHandle } from '../CanvasHandle'
import { useNodeScale } from '../useNodeScale'
import { NOTES_NODE } from './constants'
import { NotesNodeHeader } from './NotesNodeHeader'
import { NotesNodeContent } from './NotesNodeContent'
import type { NotesNodeData } from './types'
import { nodeDataEqual } from '../mappers'
import { CanvasNodeKind } from '../canvasKinds'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'

function NotesNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const nodeData = data as NotesNodeData
  const highlightMode = useProtocolHighlight(CanvasNodeKind.NOTES, id, nodeData.protocolStepId)
  const accentColor = NOTES_NODE.ACCENT_COLOR
  const [hovered, setHovered] = useState(false)
  const { containerRef, scaleFactor } = useNodeScale()
  const highlight = getNodeHighlightStyles({
    selected: selected === true,
    accentColor,
    highlightMode,
    themeMode: theme.palette.mode,
    variant: 'resizable',
  })

  return (
    <Box
      ref={containerRef}
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
      <CanvasHandle type="target" position={Position.Bottom} id="notes-input" color={accentColor} variant="passive" />

      <NodeResizer
        isVisible={hovered || selected === true}
        minWidth={NOTES_NODE.MIN_WIDTH}
        minHeight={NOTES_NODE.MIN_HEIGHT}
        maxWidth={NOTES_NODE.MAX_WIDTH}
        maxHeight={NOTES_NODE.MAX_HEIGHT}
        lineStyle={{ borderColor: 'transparent', borderWidth: 0 }}
        handleStyle={{
          width: 10,
          height: 10,
          borderRadius: 2,
          backgroundColor: accentColor,
          borderColor: accentColor,
          opacity: 0.6,
        }}
      />

      <Box sx={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden', zoom: scaleFactor }}>
        <Box
          sx={{
            height: NOTES_NODE.HEADER_HEIGHT,
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
          <NotesNodeHeader name={nodeData.label} stepName={nodeData.stepName} accentColor={accentColor} />
        </Box>

        <Box className="nowheel nodrag nopan" sx={{ flex: 1, minHeight: 0, overflow: 'hidden', position: 'relative' }}>
          <NotesNodeContent content={nodeData.content} />
        </Box>
      </Box>
    </Box>
  )
}

const notesNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const NotesNode = memo(NotesNodeComponent, notesNodeEqual)

export { NotesNode }
