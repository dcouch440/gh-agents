import { memo, useState, useCallback } from 'react'
import { Position, NodeResizer } from '@xyflow/react'
import type { NodeProps, ResizeParams } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, shareStore } from '@/stores'
import { CanvasHandle } from '../CanvasHandle'
import { SharePickerPanel } from '../SharePickerPanel'
import { useNodeScale } from '../useNodeScale'
import { DOCUMENT_NODE } from './constants'
import { DocumentNodeHeader } from './DocumentNodeHeader'
import { DocumentNodeContent } from './DocumentNodeContent'
import type { DocumentNodeData } from './types'
import { nodeDataEqual } from '../mappers'
import { setStoredDimensions } from '../nodeResizeStorage'
import { CanvasNodeKind } from '../canvasKinds'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'

function DocumentNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const nodeData = data as DocumentNodeData
  const highlightMode = useProtocolHighlight(CanvasNodeKind.DOCUMENT, id, nodeData.protocolStepId)
  const isShareSource = useStore(shareStore.store, (s) => s.active && s.sourceStepId === id)
  const accentColor = DOCUMENT_NODE.ACCENT_COLOR
  const [hovered, setHovered] = useState(false)
  const { containerRef, scaleFactor } = useNodeScale()
  const highlight = getNodeHighlightStyles({
    selected: selected === true,
    accentColor,
    highlightMode,
    themeMode: theme.palette.mode,
    variant: 'resizable',
  })

  const handleResizeEnd = useCallback(
    (_event: unknown, params: ResizeParams) => {
      setStoredDimensions(id, { width: Math.round(params.width), height: Math.round(params.height) })
    },
    [id],
  )

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
      <CanvasHandle type="target" position={Position.Bottom} id="document-input" color={accentColor} variant="passive" />

      <NodeResizer
        isVisible={hovered || selected === true}
        minWidth={DOCUMENT_NODE.MIN_WIDTH}
        minHeight={DOCUMENT_NODE.MIN_HEIGHT}
        maxWidth={DOCUMENT_NODE.MAX_WIDTH}
        maxHeight={DOCUMENT_NODE.MAX_HEIGHT}
        onResizeEnd={handleResizeEnd}
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

      {/* Zoomed inner container — scales content with node size */}
      <Box sx={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden', zoom: scaleFactor }}>
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

        {/* Content area — read-only or share overlay */}
        <Box className="nowheel nodrag nopan" sx={{ flex: 1, minHeight: 0, overflow: 'hidden', position: 'relative' }}>
          {isShareSource ? (
            <SharePickerPanel stepId={id} />
          ) : (
            <DocumentNodeContent content={nodeData.content} accentColor={accentColor} />
          )}
        </Box>
      </Box>
    </Box>
  )
}

const documentNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const DocumentNode = memo(DocumentNodeComponent, documentNodeEqual)

export { DocumentNode }
