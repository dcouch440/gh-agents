import { memo, useCallback } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore, shareStore } from '@/stores'
import { SharePickerPanel } from '../SharePickerPanel'
import { CanvasHandle } from '../CanvasHandle'
import { STEP_TYPE_COLORS, GREYSCALE_ACCENT, DetailLevel } from '../constants'
import { INPUT_NODE } from './constants'
import { InputNodeIcon } from '../Icons/InputNodeIcon'
import { ProtocolBadge } from '../ProtocolBadge'
import { NodeHeader } from '../execution'
import { InputNodeRunButton } from './InputNodeRunButton'
import { ContextNodeContent } from '../ContextNode/ContextNodeContent'
import type { InputNodeData } from './types'
import { nodeDataEqual } from '../mappers'
import { CanvasNodeKind } from '../canvasKinds'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'
import { useCanvasLOD } from '../useCanvasLOD'
import { MinimalNodeShell } from '../MinimalNodeShell'
import { ResizableNodeShell, toConstraints } from '../ResizableNodeShell'

function InputNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const detailLevel = useCanvasLOD()
  const nodeData = data as InputNodeData
  const highlightMode = useProtocolHighlight(CanvasNodeKind.INPUT, id, nodeData.protocolStepId)
  const accentColor = STEP_TYPE_COLORS['input'] ?? GREYSCALE_ACCENT
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
        <CanvasHandle type="source" position={Position.Right} color={accentColor} />
      </Box>
    )
  }

  return (
    <ResizableNodeShell
      nodeId={id}
      selected={selected === true}
      accentColor={accentColor}
      highlight={highlight}
      constraints={toConstraints(INPUT_NODE)}
      handles={<CanvasHandle type="source" position={Position.Right} color={accentColor} />}
    >
      {/* Header — draggable area */}
      <Box
        sx={{
          height: INPUT_NODE.HEADER_HEIGHT,
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
          icon={<InputNodeIcon color={accentColor} size={18} />}
          title={nodeData.label}
          subtitle="Editable input for each run"
          accentColor={accentColor}
          actions={<Box className="nodrag"><InputNodeRunButton stepId={id} /></Box>}
          badge={<ProtocolBadge color={accentColor} label="Input" animated />}
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

const inputNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const InputNode = memo(InputNodeComponent, inputNodeEqual)

export { InputNode }
