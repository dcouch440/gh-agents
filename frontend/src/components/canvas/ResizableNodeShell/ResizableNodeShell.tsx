import { useCallback } from 'react'
import { NodeResizer } from '@xyflow/react'
import type { ResizeParams } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore, canvasStore } from '@/stores'
import { useNodeScale } from '../useNodeScale'
import { isVirtualNode, setStoredDimensions } from '../nodeResizeStorage'
import type { ResizableNodeShellProps } from './types'

function ResizableNodeShell({
  nodeId,
  selected,
  accentColor,
  highlight,
  constraints,
  children,
  handles,
}: ResizableNodeShellProps) {
  const theme = useTheme()
  const hovered = useStore(canvasStore.store, (s) => s.hoveredStepId === nodeId)
  const { containerRef, scaleFactor } = useNodeScale()

  const handleResizeEnd = useCallback(
    (_event: unknown, params: ResizeParams) => {
      const width = Math.round(params.width)
      const height = Math.round(params.height)
      if (isVirtualNode(nodeId)) {
        setStoredDimensions(nodeId, { width, height })
      } else {
        void workflowStore.updateStep(nodeId, { width, height })
      }
    },
    [nodeId],
  )

  return (
    <Box
      ref={containerRef}
      sx={{
        width: '100%',
        height: '100%',
        borderRadius: '12px',
        backgroundColor: theme.palette.mode === 'light' ? theme.palette.custom.cavityBg : 'background.paper',
        border: 2,
        borderColor: highlight.borderColor,
        boxShadow: highlight.boxShadow,
        transition: 'border-color 150ms ease',
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'column',
        cursor: 'default',
      }}
    >
      <NodeResizer
        isVisible={hovered || selected}
        minWidth={constraints.minWidth}
        minHeight={constraints.minHeight}
        maxWidth={constraints.maxWidth}
        maxHeight={constraints.maxHeight}
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
      <Box sx={{
        flex: 1,
        minHeight: 0,
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        zoom: scaleFactor,
        contain: 'layout style paint',
      }}>
        {children}
      </Box>

      {handles}
    </Box>
  )
}

export { ResizableNodeShell }
