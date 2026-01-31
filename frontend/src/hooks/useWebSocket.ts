import { useContext } from 'react'
import { WebSocketContext } from '../contexts/WebSocketContext'

const useWebSocket = () => {
  const ctx = useContext(WebSocketContext)
  if (!ctx) throw new Error('useWebSocket must be used within WebSocketProvider')
  return ctx
}

export { useWebSocket }
