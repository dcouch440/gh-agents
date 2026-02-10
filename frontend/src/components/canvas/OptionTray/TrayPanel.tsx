import type { ReactNode } from 'react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { OPTION_TRAY } from './constants'

type TrayPanelProps = {
  visible: boolean
  dirty: boolean
  children: ReactNode
}

function TrayPanel({ visible, dirty, children }: TrayPanelProps) {
  const theme = useTheme()
  const isDark = theme.palette.mode === 'dark'

  if (!visible) return null

  return (
    <Box
      sx={{
        position: 'absolute',
        bottom: OPTION_TRAY.PANEL_BOTTOM,
        left: 0,
        right: 0,
        display: 'flex',
        justifyContent: 'center',
        zIndex: 10,
        pointerEvents: 'none',
      }}
    >
      <Box
        data-testid={dirty ? 'save-discard-bar' : undefined}
        sx={{
          pointerEvents: 'auto',
          display: 'flex',
          alignItems: 'center',
          gap: 1.5,
          px: 2,
          py: 1.25,
          borderRadius: `${OPTION_TRAY.PANEL_BORDER_RADIUS}px`,
          backgroundColor: theme.palette.custom.floatingPanelBg,
          backdropFilter: 'blur(16px)',
          border: '1px solid',
          borderColor: theme.palette.custom.floatingPanelBorder,
          boxShadow: isDark
            ? '0 8px 32px rgba(0, 0, 0, 0.5), 0 1px 2px rgba(0, 0, 0, 0.3)'
            : '0 8px 32px rgba(45, 27, 14, 0.12), 0 1px 2px rgba(45, 27, 14, 0.06)',
        }}
      >
        {children}
      </Box>
    </Box>
  )
}

export { TrayPanel }
export type { TrayPanelProps }
