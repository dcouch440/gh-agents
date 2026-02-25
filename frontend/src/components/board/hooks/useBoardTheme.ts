import { useTheme } from '@mui/material/styles'

type BoardTheme = {
  readonly canvasBg: string
  readonly gridDotColor: string
  readonly connectorColor: string
  readonly strokeColor: string
  readonly surfaceBg: string
  readonly accent: string
  readonly textPrimary: string
}

/**
 * Extract canvas theme tokens from the MUI theme.
 *
 * Returns the specific color values needed by the board canvas components
 * (Grid, EditableBox, ArrowLayer) so they don't need to import useTheme
 * themselves.
 */
const useBoardTheme = (): BoardTheme => {
  const theme = useTheme()
  return {
    canvasBg: theme.palette.custom.canvasBg,
    gridDotColor: theme.palette.custom.gridDotColor,
    connectorColor: theme.palette.custom.connectorColor,
    strokeColor: theme.palette.custom.strokeColor,
    surfaceBg: theme.palette.custom.surfaceBg,
    accent: theme.palette.custom.accent,
    textPrimary: theme.palette.text.primary,
  }
}

export { useBoardTheme }
export type { BoardTheme }
