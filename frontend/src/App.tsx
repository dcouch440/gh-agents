import { RouterProvider } from 'react-router-dom'
import { ThemeProvider } from '@mui/material/styles'
import CssBaseline from '@mui/material/CssBaseline'
import { AuthProvider, WebSocketProvider, OutputSchemaProvider, ReviewQueueProvider } from './contexts'
import { router } from './router'
import { theme } from './theme'

function App() {
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <AuthProvider>
        <WebSocketProvider>
          <ReviewQueueProvider>
            <OutputSchemaProvider>
              <RouterProvider router={router} />
            </OutputSchemaProvider>
          </ReviewQueueProvider>
        </WebSocketProvider>
      </AuthProvider>
    </ThemeProvider>
  )
}

export { App }
