import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import CloseOutlined from '@mui/icons-material/CloseOutlined'
import { useTheme } from '@mui/material/styles'
import { ANIMATION, FOCUS_MODE } from '@/constants'

type DetailShellProps = {
  title: string
  accentColor: string
  onClose: () => void
  children: React.ReactNode
}

function DetailShell({ title, accentColor, onClose, children }: DetailShellProps) {
  const theme = useTheme()

  return (
    <Box
      sx={{
        position: 'absolute',
        inset: 0,
        zIndex: 2,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        backgroundColor: theme.palette.background.default,
        animation: `slideDown ${ANIMATION.NORMAL}ms ease`,
        '@keyframes slideDown': {
          from: { opacity: 0, transform: 'translateY(-16px)' },
          to: { opacity: 1, transform: 'translateY(0)' },
        },
      }}
    >
      {/* Centered content column */}
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          width: '100%',
          maxWidth: FOCUS_MODE.CONTENT_MAX_WIDTH,
          flex: 1,
          minHeight: 0,
        }}
      >
        {/* Header */}
        <Box
          sx={{
            height: 52,
            display: 'flex',
            alignItems: 'center',
            gap: 1.5,
            px: 3,
            borderBottom: 1,
            borderColor: 'divider',
            backgroundColor: theme.palette.custom.bgHeader,
            flexShrink: 0,
          }}
        >
          <Box sx={{ width: 4, height: 24, borderRadius: 2, backgroundColor: accentColor, flexShrink: 0 }} />
          <Typography sx={{ fontSize: 16, fontWeight: 600, color: 'text.primary', flex: 1, minWidth: 0 }}>
            {title}
          </Typography>
          <IconButton onClick={onClose} size="small" sx={{ width: 32, height: 32, color: 'text.secondary' }}>
            <CloseOutlined sx={{ fontSize: 18 }} />
          </IconButton>
        </Box>

        {/* Content */}
        <Box sx={{ flex: 1, overflow: 'auto', p: 3 }}>
          {children}
        </Box>
      </Box>
    </Box>
  )
}

export { DetailShell }
export type { DetailShellProps }
