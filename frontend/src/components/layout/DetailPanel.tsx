import { type ReactNode, useCallback, useEffect, useRef } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import CloseRounded from '@mui/icons-material/CloseRounded'
import { useTheme } from '@mui/material/styles'
import { LAYOUT, ANIMATION } from '@/constants'

type DetailPanelProps = {
  side: 'left' | 'right'
  isOpen: boolean
  onClose: () => void
  title: string
  children: ReactNode
  width?: number
  isDragging?: boolean
  onResize?: (width: number) => void
  onDragStart?: () => void
  onDragEnd?: () => void
}

function DetailPanel({
  side,
  isOpen,
  onClose,
  title,
  children,
  width,
  isDragging = false,
  onResize,
  onDragStart,
  onDragEnd,
}: DetailPanelProps) {
  const theme = useTheme()
  const isLeft = side === 'left'
  const panelWidth = width ?? LAYOUT.PANEL_WIDTH
  const startXRef = useRef(0)
  const startWidthRef = useRef(0)
  const listenersRef = useRef<{ move: (e: MouseEvent) => void; up: () => void } | null>(null)

  const cleanup = useCallback(() => {
    if (listenersRef.current) {
      document.removeEventListener('mousemove', listenersRef.current.move)
      document.removeEventListener('mouseup', listenersRef.current.up)
      listenersRef.current = null
    }
    document.body.style.userSelect = ''
    document.body.style.cursor = ''
    onDragEnd?.()
  }, [onDragEnd])

  const startDrag = useCallback(
    (e: React.MouseEvent) => {
      if (!onResize) return
      e.preventDefault()
      cleanup()
      onDragStart?.()
      startXRef.current = e.clientX
      startWidthRef.current = panelWidth
      document.body.style.userSelect = 'none'
      document.body.style.cursor = 'col-resize'

      const move = (ev: MouseEvent) => {
        const delta = isLeft ? ev.clientX - startXRef.current : startXRef.current - ev.clientX
        const next = Math.max(LAYOUT.PANEL_MIN_WIDTH, Math.min(LAYOUT.PANEL_MAX_WIDTH, startWidthRef.current + delta))
        onResize(next)
      }

      const up = () => {
        cleanup()
      }

      listenersRef.current = { move, up }
      document.addEventListener('mousemove', move)
      document.addEventListener('mouseup', up)
    },
    [isLeft, onResize, panelWidth, cleanup, onDragStart],
  )

  useEffect(() => {
    return () => {
      cleanup()
    }
  }, [cleanup])

  return (
    <Box
      sx={{
        width: isOpen ? panelWidth : 0,
        minWidth: isOpen ? panelWidth : 0,
        height: '100%',
        overflow: 'hidden',
        transition: isDragging ? 'none' : `all ${ANIMATION.NORMAL}ms ease`,
        borderRight: isLeft ? 1 : 0,
        borderLeft: isLeft ? 0 : 1,
        borderColor: isOpen ? 'divider' : 'transparent',
        backgroundColor: theme.palette.custom.bgPanel,
        display: 'flex',
        flexDirection: 'column',
        position: 'relative',
      }}
    >
      {/* Resize handle */}
      {isOpen && onResize && (
        <Box
          onMouseDown={startDrag}
          sx={{
            position: 'absolute',
            top: 0,
            bottom: 0,
            ...(isLeft ? { right: -2 } : { left: -2 }),
            width: 4,
            cursor: 'col-resize',
            zIndex: 10,
            '&:hover': {
              backgroundColor: 'primary.main',
              opacity: 0.4,
            },
          }}
        />
      )}

      {/* Header */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          px: 1.5,
          py: 1,
          minHeight: 44,
          borderBottom: 1,
          borderColor: 'divider',
          backgroundColor: theme.palette.custom.bgHeader,
          opacity: isOpen ? 1 : 0,
          transition: `opacity ${ANIMATION.FAST}ms ease`,
        }}
      >
        <Typography
          variant="body2"
          sx={{
            fontWeight: 600,
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            color: 'text.primary',
          }}
        >
          {title}
        </Typography>
        <IconButton
          onClick={onClose}
          size="small"
          sx={{
            width: 24,
            height: 24,
            borderRadius: '50%',
            color: 'text.disabled',
            backgroundColor: 'transparent',
            transition: `all ${ANIMATION.FAST}ms ease`,
            '&:hover': {
              color: 'text.secondary',
              backgroundColor: theme.palette.custom.activeTintStrong,
              transform: 'scale(1.05)',
            },
            '&:active': {
              transform: 'scale(0.95)',
              backgroundColor: theme.palette.custom.borderHover,
            },
          }}
        >
          <CloseRounded sx={{ fontSize: 14 }} />
        </IconButton>
      </Box>

      {/* Content — zero padding for edge-to-edge children */}
      <Box
        sx={{
          flexGrow: 1,
          overflow: 'auto',
          opacity: isOpen ? 1 : 0,
          transition: `opacity ${ANIMATION.FAST}ms ease`,
        }}
      >
        {children}
      </Box>
    </Box>
  )
}

export { DetailPanel }
export type { DetailPanelProps }
