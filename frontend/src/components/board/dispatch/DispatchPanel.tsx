import { useState, useCallback, useEffect, useRef } from 'react'
import Paper from '@mui/material/Paper'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import CloseIcon from '@mui/icons-material/Close'
import { DispatchTab } from './DispatchTab'
import { RunTab } from './RunTab'

type DispatchPanelProps = {
  readonly onClose: () => void
}

type PanelTab = 'dispatch' | 'run'

const DEFAULT_WIDTH = 400
const MIN_WIDTH = 300
const MAX_WIDTH = 1200

/**
 * Side panel overlay with tabbed navigation between Dispatch and Run activity.
 */
function DispatchPanel({ onClose }: DispatchPanelProps) {
  const [activeTab, setActiveTab] = useState<PanelTab>('dispatch')
  const [width, setWidth] = useState(DEFAULT_WIDTH)

  const containerRef = useRef<HTMLDivElement>(null)
  const startXRef = useRef(0)
  const startWidthRef = useRef(0)
  const listenersRef = useRef<{ move: (e: MouseEvent) => void; up: () => void } | null>(null)

  const clampWidth = (w: number): number => Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, w))

  const cleanup = useCallback(() => {
    if (listenersRef.current) {
      document.removeEventListener('mousemove', listenersRef.current.move)
      document.removeEventListener('mouseup', listenersRef.current.up)
      listenersRef.current = null
    }
    document.body.style.userSelect = ''
    document.body.style.cursor = ''
  }, [])

  const startDrag = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()
      cleanup()
      startXRef.current = e.clientX
      startWidthRef.current = width
      document.body.style.userSelect = 'none'
      document.body.style.cursor = 'col-resize'

      const move = (ev: MouseEvent) => {
        const el = containerRef.current
        if (el === null) return
        const delta = startXRef.current - ev.clientX
        const clamped = clampWidth(startWidthRef.current + delta)
        el.style.width = `${clamped}px`
      }

      const up = () => {
        const el = containerRef.current
        if (el !== null) {
          setWidth(el.getBoundingClientRect().width)
        }
        cleanup()
      }

      listenersRef.current = { move, up }
      document.addEventListener('mousemove', move)
      document.addEventListener('mouseup', up)
    },
    [cleanup, width],
  )

  useEffect(() => {
    return () => { cleanup() }
  }, [cleanup])

  return (
    <Paper
      ref={containerRef}
      elevation={8}
      sx={{
        position: 'absolute',
        top: 0,
        right: 0,
        bottom: 0,
        width,
        zIndex: 20,
        display: 'flex',
        flexDirection: 'column',
        bgcolor: 'background.paper',
        borderLeft: 1,
        borderColor: 'divider',
        overflow: 'hidden',
      }}
    >
      {/* Resize handle (left edge) */}
      <Box
        onMouseDown={startDrag}
        sx={{
          position: 'absolute',
          top: 0,
          bottom: 0,
          left: -2,
          width: 4,
          cursor: 'col-resize',
          zIndex: 10,
          '&:hover': {
            backgroundColor: 'primary.main',
            opacity: 0.4,
          },
        }}
      />
      {/* Header */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          px: 1.5,
          py: 1,
          flexShrink: 0,
        }}
      >
        <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
          Activity
        </Typography>
        <IconButton size="small" onClick={onClose} aria-label="Close panel">
          <CloseIcon sx={{ fontSize: 16 }} />
        </IconButton>
      </Box>

      {/* Tab bar */}
      <Box sx={{ display: 'flex', borderBottom: 1, borderColor: 'divider', flexShrink: 0 }}>
        <TabButton label="Dispatch" active={activeTab === 'dispatch'} onClick={() => setActiveTab('dispatch')} />
        <TabButton label="Run" active={activeTab === 'run'} onClick={() => setActiveTab('run')} />
      </Box>

      {/* Content */}
      <Box sx={{ flex: 1, minHeight: 0, overflowY: 'auto' }}>
        {activeTab === 'dispatch' ? <DispatchTab /> : <RunTab />}
      </Box>
    </Paper>
  )
}

// ── Tab button ───────────────────────────────────────────────────────────────

type TabButtonProps = {
  readonly label: string
  readonly active: boolean
  readonly onClick: () => void
}

function TabButton({ label, active, onClick }: TabButtonProps) {
  return (
    <Box
      onClick={onClick}
      sx={{
        flex: 1,
        textAlign: 'center',
        py: 0.75,
        cursor: 'pointer',
        fontSize: 12,
        fontWeight: active ? 600 : 400,
        color: active ? 'text.primary' : 'text.secondary',
        borderBottom: 2,
        borderColor: active ? 'primary.main' : 'transparent',
        '&:hover': { bgcolor: 'action.hover' },
        userSelect: 'none',
      }}
    >
      {label}
    </Box>
  )
}

export { DispatchPanel }
export type { DispatchPanelProps }
