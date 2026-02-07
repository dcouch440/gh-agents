import { useEffect } from 'react'
import { useWebSocket } from '@/hooks/useWebSocket'
import { WS_TOPIC } from '@/types/ws'
import { sessionStore } from '@/stores/sessionStore'
import { roomStore } from '@/stores/roomStore'
import { workflowExecutionStore } from '@/stores/workflowExecutionStore'

function WsStoreRouter() {
  const { subscribe } = useWebSocket()

  useEffect(() => {
    const unsubs = [
      subscribe(WS_TOPIC.SESSION, sessionStore.handleWsEvent),
      subscribe(WS_TOPIC.ROOM, roomStore.handleWsEvent),
      subscribe(WS_TOPIC.WORKFLOW, workflowExecutionStore.handleWsEvent),
    ]
    return () => { unsubs.forEach((fn) => fn()) }
  }, [subscribe])

  return null
}

export { WsStoreRouter }
