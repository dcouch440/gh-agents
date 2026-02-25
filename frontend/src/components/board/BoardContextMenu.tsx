import { useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import type { SxProps, Theme } from '@mui/material/styles'

type MenuPosition = {
  readonly x: number
  readonly y: number
  readonly elementId: string | null
}

type BoardContextMenuProps = {
  readonly position: MenuPosition
  readonly onDelete: () => void
  readonly onSelectAll: () => void
  readonly onClose: () => void
}

const VIEWPORT_PADDING = 8

const MENU_ITEM_SX: SxProps<Theme> = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: 2,
  px: 1.5,
  py: 0.75,
  cursor: 'pointer',
  '&:hover': { backgroundColor: 'action.hover' },
}

const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.userAgent)
const modKey = isMac ? '\u2318' : 'Ctrl+'

function BoardContextMenu({ position, onDelete, onSelectAll, onClose }: BoardContextMenuProps) {
  const menuRef = useCallback((node: HTMLDivElement | null) => {
    if (node === null) return
    const rect = node.getBoundingClientRect()
    const vw = window.innerWidth
    const vh = window.innerHeight
    if (rect.right > vw - VIEWPORT_PADDING) {
      node.style.left = `${vw - rect.width - VIEWPORT_PADDING}px`
    }
    if (rect.bottom > vh - VIEWPORT_PADDING) {
      node.style.top = `${vh - rect.height - VIEWPORT_PADDING}px`
    }
  }, [])

  return (
    <Box
      ref={menuRef}
      data-testid="board-context-menu"
      onMouseDown={(e) => { e.stopPropagation() }}
      onPointerDown={(e) => { e.stopPropagation() }}
      sx={{
        position: 'fixed',
        left: position.x,
        top: position.y,
        zIndex: 1000,
        backgroundColor: 'background.paper',
        border: 1,
        borderColor: 'divider',
        borderRadius: '8px',
        boxShadow: (theme) => (theme.palette.mode === 'dark' ? '0 4px 24px rgba(0, 0, 0, 0.4)' : '0 4px 24px rgba(45, 27, 14, 0.14)'),
        minWidth: 160,
        py: 0.5,
      }}
    >
      {position.elementId !== null && (
        <>
          <Box
            data-testid="ctx-delete"
            onClick={() => { onDelete(); onClose() }}
            sx={MENU_ITEM_SX}
          >
            <Typography sx={{ fontSize: 12, color: 'error.main' }}>Delete</Typography>
            <Typography sx={{ fontSize: 11, color: 'text.secondary' }}>Del</Typography>
          </Box>
          <Box sx={{ mx: 1.5, my: 0.5, borderTop: 1, borderColor: 'divider' }} />
        </>
      )}
      <Box
        data-testid="ctx-select-all"
        onClick={() => { onSelectAll(); onClose() }}
        sx={MENU_ITEM_SX}
      >
        <Typography sx={{ fontSize: 12, color: 'text.primary' }}>Select All</Typography>
        <Typography sx={{ fontSize: 11, color: 'text.secondary' }}>{modKey}A</Typography>
      </Box>
    </Box>
  )
}

export { BoardContextMenu }
export type { MenuPosition }
