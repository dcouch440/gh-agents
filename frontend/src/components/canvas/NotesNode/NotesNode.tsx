import { memo } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { CanvasHandle } from '../CanvasHandle'
import { DetailLevel } from '../constants'
import { NOTES_NODE } from './constants'
import { NotesIcon } from '../Icons/NotesIcon'
import { ProtocolBadge } from '../ProtocolBadge'
import { NodeHeader } from '../execution'
import { NotesNodeContent } from './NotesNodeContent'
import type { NotesNodeData } from './types'
import { nodeDataEqual } from '../mappers'
import { CanvasNodeKind } from '../canvasKinds'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'
import { useCanvasLOD } from '../useCanvasLOD'
import { MinimalNodeShell } from '../MinimalNodeShell'
import { ResizableNodeShell, toConstraints } from '../ResizableNodeShell'

function NotesNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const detailLevel = useCanvasLOD()
  const nodeData = data as NotesNodeData
  const highlightMode = useProtocolHighlight(CanvasNodeKind.NOTES, id, nodeData.protocolStepId)
  const accentColor = NOTES_NODE.ACCENT_COLOR
  const highlight = getNodeHighlightStyles({
    selected: selected === true,
    accentColor,
    highlightMode,
    themeMode: theme.palette.mode,
    variant: 'resizable',
  })

  if (detailLevel === DetailLevel.MINIMAL) {
    return (
      <Box sx={{ width: '100%', height: '100%' }}>
        <MinimalNodeShell
          label={nodeData.label}
          accentColor={accentColor}
          borderColor={highlight.borderColor}
          boxShadow={highlight.boxShadow}
        />
        <CanvasHandle type="target" position={Position.Top} id="notes-input" color={accentColor} variant="passive" />
      </Box>
    )
  }

  return (
    <ResizableNodeShell
      nodeId={id}
      selected={selected === true}
      accentColor={accentColor}
      highlight={highlight}
      constraints={toConstraints(NOTES_NODE)}
      handles={<CanvasHandle type="target" position={Position.Top} id="notes-input" color={accentColor} variant="passive" />}
    >
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
        <NodeHeader
          icon={<NotesIcon color={accentColor} size={18} />}
          title={nodeData.label}
          subtitle={nodeData.stepName}
          accentColor={accentColor}
          badge={<ProtocolBadge color={accentColor} label="Notes" />}
        />
      </Box>

      <Box className="nowheel nodrag nopan" sx={{ flex: 1, minHeight: 0, overflow: 'hidden', position: 'relative' }}>
        <NotesNodeContent content={nodeData.content} />
      </Box>
    </ResizableNodeShell>
  )
}

const notesNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const NotesNode = memo(NotesNodeComponent, notesNodeEqual)

export { NotesNode }
