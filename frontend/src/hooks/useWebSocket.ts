import { useContext } from 'react'
import { WebSocketContext } from '@/contexts/WebSocketContext'
import { useStore } from '@/stores/lib'
import { wsConnectionStore } from '@/stores/wsConnectionStore'

const useWebSocket = () => {
  const ctx = useContext(WebSocketContext)
  if (!ctx) throw new Error('useWebSocket must be used within WebSocketProvider')
  const status = useStore(wsConnectionStore, (s) => s.status)
  const latency = useStore(wsConnectionStore, (s) => s.latency)
  return { ...ctx, status, latency }
}

export { useWebSocket }
