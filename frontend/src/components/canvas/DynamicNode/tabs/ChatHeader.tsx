import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import DeleteOutlined from '@mui/icons-material/DeleteOutlined'

type ChatHeaderProps = {
  onClear: () => void
  disabled: boolean
}

function ChatHeader({ onClear, disabled }: ChatHeaderProps) {
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
      <IconButton size="small" onClick={onClear} disabled={disabled} sx={{ p: 0.25 }}>
        <DeleteOutlined sx={{ fontSize: 14 }} />
      </IconButton>
    </Box>
  )
}

export { ChatHeader }
export type { ChatHeaderProps }
