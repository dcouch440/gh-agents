import { memo, useState, useCallback } from 'react'
import { Position, NodeResizer } from '@xyflow/react'
import type { ResizeParams } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { CanvasHandle } from '../CanvasHandle'
import { HighlightMode } from '../canvasKinds'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'
import { useNodeScale } from '../useNodeScale'
import { FORM_NODE } from './constants'
import { FormTabStrip } from './FormTabStrip'
import type { CanvasFormNodeProps } from './types'

function CanvasFormNodeComponent({
  header,
  headerHeight = FORM_NODE.HEADER_HEIGHT,
  tabs,
  activeTabId,
  onTabChange,
  selected,
  accentColor,
  highlightMode = HighlightMode.NONE,
  extraHandles,
  overlay,
  onResizeEnd: onResizeEndProp,
}: CanvasFormNodeProps) {
  const theme = useTheme()
  const resolvedAccent = accentColor ?? theme.palette.primary.main
  const highlight = getNodeHighlightStyles({
    selected,
    accentColor: resolvedAccent,
    highlightMode,
    themeMode: theme.palette.mode,
    variant: 'resizable',
  })
  const activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0]
  const [hovered, setHovered] = useState(false)
  const { containerRef, scaleFactor } = useNodeScale()

  const handleResizeEnd = useCallback(
    (_event: unknown, params: ResizeParams) => {
      onResizeEndProp?.(Math.round(params.width), Math.round(params.height))
    },
    [onResizeEndProp],
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
        isVisible={hovered || selected}
        minWidth={FORM_NODE.MIN_WIDTH}
        minHeight={FORM_NODE.MIN_HEIGHT}
        maxWidth={FORM_NODE.MAX_WIDTH}
        maxHeight={FORM_NODE.MAX_HEIGHT}
        onResizeEnd={handleResizeEnd}
        lineStyle={{
          borderColor: 'transparent',
          borderWidth: 0,
        }}
        handleStyle={{
          width: 10,
          height: 10,
          borderRadius: 2,
          backgroundColor: resolvedAccent,
          borderColor: resolvedAccent,
          opacity: 0.6,
        }}
      />

      {/* Zoomed inner container — scales content with node size */}
      <Box sx={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden', zoom: scaleFactor }}>
        {/* Header slot — draggable area */}
        {header !== null && (
          <Box
            sx={{
              height: headerHeight,
              overflow: 'hidden',
              borderBottom: 1,
              borderColor: 'divider',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              backgroundColor: theme.palette.custom.bgHeader,
              flexShrink: 0,
              cursor: 'grab',
              '&:active': { cursor: 'grabbing' },
            }}
          >
            {header}
          </Box>
        )}

        {overlay ? (
          <Box className="nowheel nodrag nopan" sx={{ flex: 1, minHeight: 0, overflow: 'hidden', position: 'relative' }}>
            {overlay}
          </Box>
        ) : (
          <>
            <FormTabStrip tabs={tabs} activeTabId={activeTabId} onTabChange={onTabChange} accentColor={resolvedAccent} />
            {/* Content area — full-bleed, no padding, interactive */}
            <Box className="nowheel nodrag nopan" sx={{ flex: 1, minHeight: 0, overflow: 'hidden', position: 'relative', cursor: 'text', userSelect: 'text' }}>
              {activeTab?.content}
            </Box>
          </>
        )}
      </Box>

      {/* Handles */}
      <CanvasHandle type="target" position={Position.Left} color={resolvedAccent} />
      <CanvasHandle type="source" position={Position.Right} color={resolvedAccent} />
      {extraHandles}
    </Box>
  )
}

const CanvasFormNode = memo(CanvasFormNodeComponent)

export { CanvasFormNode }
