import { RouterProvider } from 'react-router-dom'
import { ThemeProvider } from '@mui/material/styles'
import CssBaseline from '@mui/material/CssBaseline'
import { AuthProvider, WebSocketProvider, OutputSchemaProvider } from './contexts'
import { router } from './router'
import { theme } from './theme'

function App() {
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <AuthProvider>
        <WebSocketProvider>
          <OutputSchemaProvider>
            <RouterProvider router={router} />
          </OutputSchemaProvider>
        </WebSocketProvider>
      </AuthProvider>
    </ThemeProvider>
  )
}

export { App }
