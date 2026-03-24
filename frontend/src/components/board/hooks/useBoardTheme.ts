import { useTheme } from '@mui/material/styles'
import type { DrawTheme } from '../canvas/renderer'

/**
 * Extract canvas theme tokens from the MUI theme.
 *
 * Returns the specific color values needed by the board canvas components
 * (Grid, EditableBox, ArrowLayer) so they don't need to import useTheme
 * themselves.
 */
const useBoardTheme = (): DrawTheme => {
  const theme = useTheme()
  return {
    canvasBg: theme.palette.custom.canvasBg,
    gridDotColor: theme.palette.custom.gridDotColor,
    connectorColor: theme.palette.custom.connectorColor,
    strokeColor: theme.palette.custom.strokeColor,
    surfaceBg: theme.palette.custom.surfaceBg,
    accentColor: theme.palette.custom.accent,
    textColor: theme.palette.text.primary,
  }
}

export { useBoardTheme }
