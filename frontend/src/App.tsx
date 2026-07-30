import { useEffect, useMemo } from 'react'
import { RouterProvider } from 'react-router-dom'
import { ThemeProvider } from '@mui/material/styles'
import CssBaseline from '@mui/material/CssBaseline'
import { ThemeModeProvider, CommandPaletteProvider } from './contexts'
import { WebSocketProvider } from './contexts/WebSocketContext'
import { WsStoreRouter } from './stores/ws/WsStoreRouter'
import { useThemeMode } from './hooks/useThemeMode'
import { router } from './router'
import { createAppTheme } from './theme'
import { CommandPalette } from './components/command-palette'
import { ReviewQueueNotification } from './components/layout/ReviewQueueNotification'
import { ToastStack } from './components/primitives'
import { authStore, reviewQueueStore } from './stores'
import { setupAuthInterceptor } from './api/authInterceptor'
import { dismissSplash } from './utils/splash'

function AppInner() {
  const { themeId } = useThemeMode()
  const theme = useMemo(() => createAppTheme(themeId), [themeId])

  useEffect(() => {
    setupAuthInterceptor()
    void authStore.hydrate()
    void reviewQueueStore.fetchPending()
    dismissSplash()
  }, [])

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <WebSocketProvider>
        <WsStoreRouter />
        <CommandPaletteProvider>
          <CommandPalette />
          <RouterProvider router={router} />
        </CommandPaletteProvider>
        <ReviewQueueNotification />
        <ToastStack />
      </WebSocketProvider>
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
