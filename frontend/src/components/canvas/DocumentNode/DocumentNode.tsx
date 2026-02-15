import { memo } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, shareStore } from '@/stores'
import { CanvasHandle } from '../CanvasHandle'
import { SharePickerPanel } from '../SharePickerPanel'
import { DOCUMENT_NODE } from './constants'
import { DocumentNodeHeader } from './DocumentNodeHeader'
import { DocumentNodeContent } from './DocumentNodeContent'
import type { DocumentNodeData } from './types'
import { nodeDataEqual } from '../mappers'
import { CanvasNodeKind } from '../canvasKinds'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'
import { ResizableNodeShell, toConstraints } from '../ResizableNodeShell'

function DocumentNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const nodeData = data as DocumentNodeData
  const highlightMode = useProtocolHighlight(CanvasNodeKind.DOCUMENT, id, nodeData.protocolStepId)
  const isShareSource = useStore(shareStore.store, (s) => s.active && s.sourceStepId === id)
  const accentColor = DOCUMENT_NODE.ACCENT_COLOR
  const highlight = getNodeHighlightStyles({
    selected: selected === true,
    accentColor,
    highlightMode,
    themeMode: theme.palette.mode,
    variant: 'resizable',
  })

  return (
    <ResizableNodeShell
      nodeId={id}
      selected={selected === true}
      accentColor={accentColor}
      highlight={highlight}
      constraints={toConstraints(DOCUMENT_NODE)}
      handles={<CanvasHandle type="target" position={Position.Bottom} id="document-input" color={accentColor} variant="passive" />}
    >
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
    </ResizableNodeShell>
  )
}

const documentNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const DocumentNode = memo(DocumentNodeComponent, documentNodeEqual)

export { DocumentNode }
