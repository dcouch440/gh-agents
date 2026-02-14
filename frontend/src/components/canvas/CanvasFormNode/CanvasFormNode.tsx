import { memo, useEffect, useRef, useState } from 'react'
import { Position, NodeResizer } from '@xyflow/react'
import Box from '@mui/material/Box'
import Tooltip from '@mui/material/Tooltip'
import { useTheme } from '@mui/material/styles'
import { CanvasHandle } from '../CanvasHandle'
import { HighlightMode } from '../canvasKinds'
import { FORM_NODE } from './constants'
import { resolveScaleFactor } from './scaleNotch'
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
}: CanvasFormNodeProps) {
  const theme = useTheme()
  const resolvedAccent = accentColor ?? theme.palette.primary.main
  const activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0]
  const [hovered, setHovered] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)
  const [scaleFactor, setScaleFactor] = useState(1)

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const observer = new ResizeObserver((entries) => {
      const rect = entries[0]?.contentRect
      if (!rect) return
      const next = resolveScaleFactor(rect.width, rect.height)
      setScaleFactor((prev) => (prev === next ? prev : next))
    })
    observer.observe(el)
    return () => { observer.disconnect() }
  }, [])

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
        borderColor: selected
          ? resolvedAccent
          : highlightMode === HighlightMode.SELECT
            ? resolvedAccent
            : highlightMode === HighlightMode.HOVER
              ? `${resolvedAccent}80`
              : 'divider',
        boxShadow: selected
          ? theme.palette.mode === 'dark'
            ? `0 0 0 1px ${resolvedAccent}40, 0 8px 32px ${resolvedAccent}2E, 0 2px 8px rgba(0, 0, 0, 0.3)`
            : `0 0 0 1px ${resolvedAccent}30, 0 12px 40px rgba(45, 27, 14, 0.18), 0 4px 12px ${resolvedAccent}1E`
          : highlightMode === HighlightMode.SELECT
            ? `0 0 0 1px ${resolvedAccent}40, 0 8px 32px ${resolvedAccent}22`
            : highlightMode === HighlightMode.HOVER
              ? `0 0 0 1px ${resolvedAccent}20, 0 6px 24px ${resolvedAccent}14`
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
      <NodeResizer
        isVisible={hovered || selected}
        minWidth={FORM_NODE.MIN_WIDTH}
        minHeight={FORM_NODE.MIN_HEIGHT}
        maxWidth={FORM_NODE.MAX_WIDTH}
        maxHeight={FORM_NODE.MAX_HEIGHT}
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

        {/* Horizontal tab strip — draggable area */}
        <Box
          role="tablist"
          sx={{
            height: FORM_NODE.TAB_STRIP_HEIGHT,
            display: 'flex',
            alignItems: 'center',
            gap: 0.25,
            px: 0.5,
            borderBottom: 1,
            borderColor: 'divider',
            backgroundColor: theme.palette.custom.bgHeader,
            flexShrink: 0,
          }}
        >
          {tabs.map((tab) => {
            const isActive = tab.id === activeTabId
            const IconComponent = tab.icon
            return (
              <Tooltip key={tab.id} title={tab.tooltip} placement="bottom">
                <Box
                  data-testid={`tab-${tab.id}`}
                  onClick={() => {
                    onTabChange(tab.id)
                  }}
                  role="tab"
                  tabIndex={0}
                  aria-selected={isActive}
                  aria-label={tab.tooltip}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') onTabChange(tab.id)
                  }}
                  sx={{
                    width: 28,
                    height: 24,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    borderRadius: '4px',
                    cursor: 'pointer',
                    position: 'relative',
                    backgroundColor: isActive ? theme.palette.custom.activeTint : 'transparent',
                    transition: 'background-color 120ms ease',
                    '&:hover': isActive ? {} : { backgroundColor: theme.palette.custom.hoverOverlay },
                    ...(isActive
                      ? {
                          '&::after': {
                            content: '""',
                            position: 'absolute',
                            bottom: -4,
                            left: 4,
                            right: 4,
                            height: 2,
                            borderRadius: 1,
                            backgroundColor: resolvedAccent,
                          },
                        }
                      : {}),
                  }}
                >
                  <IconComponent
                    sx={{
                      fontSize: 16,
                      color: isActive ? resolvedAccent : 'text.secondary',
                      transition: 'color 120ms ease',
                    }}
                  />
                </Box>
              </Tooltip>
            )
          })}
        </Box>

        {/* Content area — full-bleed, no padding, interactive */}
        <Box className="nowheel nodrag nopan" sx={{ flex: 1, minHeight: 0, overflow: 'hidden', position: 'relative', cursor: 'text', userSelect: 'text' }}>
          {activeTab?.content}
        </Box>
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
