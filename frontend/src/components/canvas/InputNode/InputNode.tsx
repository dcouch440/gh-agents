import { memo, useState, useCallback } from 'react'
import { Position, NodeResizer } from '@xyflow/react'
import type { NodeProps, ResizeParams } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore, shareStore } from '@/stores'
import { SharePickerPanel } from '../SharePickerPanel'
import { CanvasHandle } from '../CanvasHandle'
import { STEP_TYPE_COLORS, GREYSCALE_ACCENT } from '../constants'
import { useNodeScale } from '../useNodeScale'
import { INPUT_NODE } from './constants'
import { InputNodeHeader } from './InputNodeHeader'
import { ContextNodeContent } from '../ContextNode/ContextNodeContent'
import type { InputNodeData } from './types'
import { nodeDataEqual } from '../mappers'
import { CanvasNodeKind } from '../canvasKinds'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'

function InputNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const nodeData = data as InputNodeData
  const highlightMode = useProtocolHighlight(CanvasNodeKind.INPUT, id, nodeData.protocolStepId)
  const accentColor = STEP_TYPE_COLORS['input'] ?? GREYSCALE_ACCENT
  const isShareSource = useStore(shareStore.store, (s) => s.active && s.sourceStepId === id)
  const [hovered, setHovered] = useState(false)
  const { containerRef, scaleFactor } = useNodeScale()
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

  const handleResizeEnd = useCallback(
    (_event: unknown, params: ResizeParams) => {
      void workflowStore.updateStep(id, {
        width: Math.round(params.width),
        height: Math.round(params.height),
      })
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
      <NodeResizer
        isVisible={hovered || selected === true}
        minWidth={INPUT_NODE.MIN_WIDTH}
        minHeight={INPUT_NODE.MIN_HEIGHT}
        maxWidth={INPUT_NODE.MAX_WIDTH}
        maxHeight={INPUT_NODE.MAX_HEIGHT}
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
          <InputNodeHeader stepId={id} name={nodeData.label} accentColor={accentColor} />
        </Box>

        {/* Content area — interactive, no drag */}
        <Box className="nowheel nodrag nopan" sx={{ flex: 1, minHeight: 0, overflow: 'hidden', position: 'relative' }}>
          {isShareSource ? (
            <SharePickerPanel stepId={id} />
          ) : (
            <ContextNodeContent content={nodeData.content} accentColor={accentColor} onChange={handleContentChange} />
          )}
        </Box>
      </Box>

      {/* Source handle only — input nodes are source-only, no target handle */}
      <CanvasHandle type="source" position={Position.Bottom} color={accentColor} />
    </Box>
  )
}

const inputNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const InputNode = memo(InputNodeComponent, inputNodeEqual)

export { InputNode }
