import { useCallback, useEffect, useRef } from 'react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { TabSelector } from '@/components/primitives'
import { useStore, sidebarStore } from '@/stores'
import { ChatPanel } from '@/components/chat/ChatPanel'
import type { ChatMessageData } from '@/components/chat/MessageList'
import { StepTree } from './StepTree'
import type { TabOption } from '@/components/primitives'

// ── Constants ───────────────────────────────────────────────────────────────

const TAB_OPTIONS: TabOption[] = [
  { value: 'tree', label: 'Tree' },
  { value: 'chat', label: 'Chat' },
]

type WorkflowSidebarProps = {
  messages: ChatMessageData[]
  onSend: (message: string) => void
  onCancel?: () => void
  streaming?: boolean
  onPanelSubmit?: (messageId: string, selections: string) => void
}

// ── Component ───────────────────────────────────────────────────────────────

function WorkflowSidebar({ messages, onSend, onCancel, streaming, onPanelSubmit }: WorkflowSidebarProps) {
  const theme = useTheme()
  const activeTab = useStore(sidebarStore.store, sidebarStore.selectActiveTab)
  const width = useStore(sidebarStore.store, sidebarStore.selectWidth)

  const containerRef = useRef<HTMLDivElement>(null)
  const startXRef = useRef(0)
  const startWidthRef = useRef(0)
  const listenersRef = useRef<{ move: (e: MouseEvent) => void; up: () => void } | null>(null)
  const draggingRef = useRef(false)

  const clampWidth = (w: number): number =>
    Math.max(sidebarStore.MIN_WIDTH, Math.min(sidebarStore.MAX_WIDTH, w))

  const cleanup = useCallback(() => {
    if (listenersRef.current) {
      document.removeEventListener('mousemove', listenersRef.current.move)
      document.removeEventListener('mouseup', listenersRef.current.up)
      listenersRef.current = null
    }
    document.body.style.userSelect = ''
    document.body.style.cursor = ''
    draggingRef.current = false
  }, [])

  const startDrag = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()
      cleanup()
      draggingRef.current = true
      startXRef.current = e.clientX
      startWidthRef.current = width
      document.body.style.userSelect = 'none'
      document.body.style.cursor = 'col-resize'

      const move = (ev: MouseEvent) => {
        const el = containerRef.current
        if (el === null) return
        const delta = startXRef.current - ev.clientX
        const clamped = clampWidth(startWidthRef.current + delta)
        // Apply directly to DOM — no React re-renders during drag
        el.style.width = `${clamped}px`
        el.style.minWidth = `${clamped}px`
      }

      const up = () => {
        // Read final width from DOM and commit to store once
        const el = containerRef.current
        if (el !== null) {
          sidebarStore.setWidth(el.getBoundingClientRect().width)
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
    <Box
      ref={containerRef}
      sx={{
        width,
        minWidth: width,
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        borderLeft: 1,
        borderColor: 'divider',
        backgroundColor: theme.palette.custom.bgPanel,
        position: 'relative',
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

      {/* Tab header */}
      <Box
        sx={{
          borderBottom: 1,
          borderColor: 'divider',
          backgroundColor: theme.palette.custom.bgHeader,
        }}
      >
        <TabSelector
          options={TAB_OPTIONS}
          value={activeTab}
          onChange={(v) => { sidebarStore.setActiveTab(v as 'tree' | 'chat') }}
        />
      </Box>

      {/* Tab content */}
      {activeTab === 'tree' ? (
        <Box sx={{ flex: 1, minHeight: 0, overflow: 'auto' }}>
          <StepTree />
        </Box>
      ) : (
        <Box sx={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
          <ChatPanel
            messages={messages}
            onSend={onSend}
            onCancel={onCancel}
            streaming={streaming}
            emptyMessage="Describe your workflow and I'll build it."
            onPanelSubmit={onPanelSubmit}
          />
        </Box>
      )}
    </Box>
  )
}

export { WorkflowSidebar }
