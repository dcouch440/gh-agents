import { useEffect } from 'react'
import { useWebSocket } from '@/hooks/useWebSocket'
import { WS_TOPIC } from '@/types/ws'
import { sessionStore } from '@/stores/sessionStore'
import { roomStore } from '@/stores/roomStore'
import { workflowExecutionStore } from '@/stores/workflowExecutionStore'
import { activityStore } from '@/stores/activity'

function WsStoreRouter() {
  const { subscribe } = useWebSocket()

  useEffect(() => {
    const unsubs = [
      // Domain store handlers
      subscribe(WS_TOPIC.SESSION, sessionStore.handleWsEvent),
      subscribe(WS_TOPIC.ROOM, roomStore.handleWsEvent),
      subscribe(WS_TOPIC.WORKFLOW, workflowExecutionStore.handleWsEvent),
      // Flight recorder — receives ALL topics
      subscribe(WS_TOPIC.SESSION, activityStore.handleWsEvent),
      subscribe(WS_TOPIC.ROOM, activityStore.handleWsEvent),
      subscribe(WS_TOPIC.WORKFLOW, activityStore.handleWsEvent),
    ]
    return () => { unsubs.forEach((fn) => fn()) }
  }, [subscribe])

  return null
}

export { WsStoreRouter }
