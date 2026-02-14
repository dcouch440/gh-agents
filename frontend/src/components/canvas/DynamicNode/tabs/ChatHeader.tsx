import { useCallback, useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import DeleteOutlined from '@mui/icons-material/DeleteOutlined'
import AddLinkOutlined from '@mui/icons-material/AddLinkOutlined'
import { Tooltip } from '@/components/primitives'
import { ContextMentionMenu } from '../../ContextMentionMenu'

type ChatHeaderProps = {
  stepId: string
  onClear: () => void
  disabled: boolean
}

function ChatHeader({ stepId, onClear, disabled }: ChatHeaderProps) {
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)

  const handleToggleMenu = useCallback((e: React.MouseEvent<HTMLElement>) => {
    setAnchorEl((prev) => (prev ? null : e.currentTarget))
  }, [])

  const handleCloseMenu = useCallback(() => {
    setAnchorEl(null)
  }, [])

  return (
    <Box
      sx={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        px: 1.5,
        py: 0.25,
        position: 'relative',
        '&::after': {
          content: '""',
          position: 'absolute',
          bottom: 0,
          left: 12,
          right: 12,
          height: '1px',
          bgcolor: 'divider',
          opacity: 0.5,
        },
      }}
    >
      <Typography sx={{ fontSize: 11, fontWeight: 500, color: 'text.secondary', opacity: 0.7 }}>Chat</Typography>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.25 }}>
        <Tooltip title={anchorEl ? 'Close context menu' : 'Add context mentions'}>
          <IconButton
            size="small"
            onClick={handleToggleMenu}
            sx={{
              p: 0.25,
              color: anchorEl ? 'primary.main' : 'text.secondary',
            }}
          >
            <AddLinkOutlined sx={{ fontSize: 14 }} />
          </IconButton>
        </Tooltip>
        <ContextMentionMenu anchorEl={anchorEl} onClose={handleCloseMenu} stepId={stepId} />
        <IconButton size="small" onClick={onClear} disabled={disabled} sx={{ p: 0.25 }}>
          <DeleteOutlined sx={{ fontSize: 14 }} />
        </IconButton>
      </Box>
    </Box>
  )
}

export { ChatHeader }
export type { ChatHeaderProps }
