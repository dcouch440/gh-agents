import { useMemo } from 'react'
import { RouterProvider } from 'react-router-dom'
import { ThemeProvider } from '@mui/material/styles'
import CssBaseline from '@mui/material/CssBaseline'
import { AuthProvider, WebSocketProvider, OutputSchemaProvider, ReviewQueueProvider, ThemeModeProvider, CommandPaletteProvider } from './contexts'
import { useThemeMode } from './hooks/useThemeMode'
import { router } from './router'
import { createAppTheme } from './theme'
import { CommandPalette } from './components/command-palette'

function AppInner() {
  const { mode } = useThemeMode()
  const theme = useMemo(() => createAppTheme(mode), [mode])

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <AuthProvider>
        <WebSocketProvider>
          <ReviewQueueProvider>
            <OutputSchemaProvider>
              <CommandPaletteProvider>
                <CommandPalette />
                <RouterProvider router={router} />
              </CommandPaletteProvider>
            </OutputSchemaProvider>
          </ReviewQueueProvider>
        </WebSocketProvider>
      </AuthProvider>
    </ThemeProvider>
  )
}

function App() {
  return (
    <ThemeModeProvider>
      <AppInner />
    </ThemeModeProvider>
  )
}

export { App }
