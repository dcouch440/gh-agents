import type { SxProps, Theme } from '@mui/material/styles'

const OPTION_TRAY = {
  TOGGLE_WIDTH: 48,
  TOGGLE_HEIGHT: 28,
  TOGGLE_BOTTOM: 16,
  PANEL_BOTTOM: 52,
  PANEL_BORDER_RADIUS: 16,
} as const

/** Shared base sx for contained tray action buttons (Save, Run). */
const TRAY_BUTTON_CONTAINED_SX: SxProps<Theme> = {
  fontSize: 13,
  fontWeight: 600,
  textTransform: 'none',
  px: 2.5,
  py: 0.75,
  color: '#fff',
  boxShadow: 'none',
}

export { OPTION_TRAY, TRAY_BUTTON_CONTAINED_SX }
