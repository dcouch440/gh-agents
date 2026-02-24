import { useTheme } from '@mui/material/styles'

/**
 * Map the app's MUI palette mode to Excalidraw's theme value.
 *
 * Excalidraw accepts `"light" | "dark"`. MUI's `palette.mode` uses the same
 * string literal union, so this is a direct pass-through today. Extracting it
 * as a hook keeps Board.tsx free of theme logic and lets us extend the mapping
 * later (e.g. Excalidraw-specific color overrides) without touching the component.
 */
const useBoardTheme = (): 'light' | 'dark' => {
  const theme = useTheme()
  return theme.palette.mode === 'light' ? 'light' : 'dark'
}

export { useBoardTheme }
