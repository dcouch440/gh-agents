import { RouterProvider } from 'react-router-dom'
import { AuthProvider, WebSocketProvider } from './contexts'
import { router } from './router'

function App() {
  return (
    <AuthProvider>
      <WebSocketProvider>
        <RouterProvider router={router} />
      </WebSocketProvider>
    </AuthProvider>
  )
}

export { App }
