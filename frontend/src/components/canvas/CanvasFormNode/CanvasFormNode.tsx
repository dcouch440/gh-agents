import { memo } from 'react'
import { Position } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { CanvasHandle } from '../CanvasHandle'
import { HighlightMode } from '../canvasKinds'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'
import { ResizableNodeShell, toConstraints } from '../ResizableNodeShell'
import { FORM_NODE } from './constants'
import { FormTabStrip } from './FormTabStrip'
import type { CanvasFormNodeProps } from './types'

function CanvasFormNodeComponent({
  nodeId,
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

  return (
    <ResizableNodeShell
      nodeId={nodeId}
      selected={selected}
      accentColor={resolvedAccent}
      highlight={highlight}
      constraints={toConstraints(FORM_NODE)}
      handles={
        <>
          <CanvasHandle type="target" position={Position.Left} color={resolvedAccent} />
          <CanvasHandle type="source" position={Position.Right} color={resolvedAccent} />
          {extraHandles}
        </>
      }
    >
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
    </ResizableNodeShell>
  )
}

const CanvasFormNode = memo(CanvasFormNodeComponent)

export { CanvasFormNode }
