import Box from '@mui/material/Box'
import ButtonBase from '@mui/material/ButtonBase'
import KeyboardArrowUp from '@mui/icons-material/KeyboardArrowUp'
import { useTheme } from '@mui/material/styles'
import { OPTION_TRAY } from './constants'

type TrayToggleProps = {
  open: boolean
  onClick: () => void
}

function TrayToggle({ open, onClick }: TrayToggleProps) {
  const theme = useTheme()
  const isDark = theme.palette.mode === 'dark'

  return (
    <Box
      sx={{
        position: 'absolute',
        bottom: OPTION_TRAY.TOGGLE_BOTTOM,
        left: '50%',
        transform: 'translateX(-50%)',
        zIndex: 10,
      }}
    >
      <ButtonBase
        data-testid="tray-toggle"
        onClick={onClick}
        sx={{
          width: OPTION_TRAY.TOGGLE_WIDTH,
          height: OPTION_TRAY.TOGGLE_HEIGHT,
          borderRadius: '100px',
          backgroundColor: theme.palette.custom.floatingPanelBg,
          backdropFilter: 'blur(16px)',
          border: '1px solid',
          borderColor: theme.palette.custom.floatingPanelBorder,
          boxShadow: isDark
            ? '0 4px 12px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.05)'
            : '0 4px 12px rgba(45, 27, 14, 0.08)',
          transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          '&:hover': {
            borderColor: theme.palette.custom.borderHover,
            transform: 'translateY(-1px)',
            boxShadow: isDark
              ? '0 6px 16px rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.08)'
              : '0 6px 16px rgba(45, 27, 14, 0.12)',
          },
        }}
      >
        <KeyboardArrowUp
          sx={{
            fontSize: 18,
            color: 'text.secondary',
            transition: 'transform 200ms cubic-bezier(0.4, 0, 0.2, 1)',
            transform: open ? 'rotate(180deg)' : 'rotate(0deg)',
          }}
        />
      </ButtonBase>
    </Box>
  )
}

export { TrayToggle }
export type { TrayToggleProps }
