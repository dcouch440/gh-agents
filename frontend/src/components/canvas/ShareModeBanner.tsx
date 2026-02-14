import { useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Button from '@mui/material/Button'
import { shareStore } from '@/stores'

function ShareModeBanner() {
  const handleCancel = useCallback(() => {
    shareStore.cancelShare()
  }, [])

  return (
    <Box
      sx={{
        position: 'absolute',
        top: 12,
        left: '50%',
        transform: 'translateX(-50%)',
        zIndex: 10,
        display: 'flex',
        alignItems: 'center',
        gap: 1.5,
        px: 2,
        py: 0.75,
        borderRadius: '8px',
        backgroundColor: 'background.paper',
        border: 1,
        borderColor: 'divider',
        boxShadow: (theme) =>
          theme.palette.mode === 'dark'
            ? '0 4px 24px rgba(0, 0, 0, 0.4)'
            : '0 4px 24px rgba(45, 27, 14, 0.14)',
        pointerEvents: 'auto',
      }}
    >
      <Typography sx={{ fontSize: 12, color: 'text.primary', fontWeight: 500 }}>
        Click a node to share context
      </Typography>
      <Button
        size="small"
        variant="text"
        onClick={handleCancel}
        sx={{
          fontSize: 11,
          textTransform: 'none',
          color: 'text.secondary',
          minWidth: 'auto',
          px: 1,
          py: 0.25,
        }}
      >
        Cancel
      </Button>
    </Box>
  )
}

export { ShareModeBanner }
