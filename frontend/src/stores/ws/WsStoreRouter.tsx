import { useEffect } from 'react'
import { useWebSocket } from '@/hooks/useWebSocket'
import { WS_TOPIC } from '@/types/ws'
import { sessionStore } from '@/stores/sessionStore'
import { dispatchStore } from '@/stores/dispatchStore'
import { roomStore } from '@/stores/roomStore'
import { workflowExecutionStore } from '@/stores/workflowExecutionStore'
import { workflowStore } from '@/stores/workflowStore'
import { stepStreamStore } from '@/stores/stepStreamStore'
import { activityStore } from '@/stores/activity'
import { agentTraceStore } from '@/stores/agentTraceStore'

function WsStoreRouter() {
  const { subscribe } = useWebSocket()

  useEffect(() => {
    const unsubs = [
      // Domain store handlers
      subscribe(WS_TOPIC.SESSION, sessionStore.handleWsEvent),
      subscribe(WS_TOPIC.SESSION, dispatchStore.handleWsEvent),
      subscribe(WS_TOPIC.ROOM, roomStore.handleWsEvent),
      subscribe(WS_TOPIC.WORKFLOW, workflowExecutionStore.handleWsEvent),
      subscribe(WS_TOPIC.WORKFLOW, workflowStore.handleWsEvent),
      subscribe(WS_TOPIC.WORKFLOW, stepStreamStore.handleWsEvent),
      subscribe(WS_TOPIC.WORKFLOW, agentTraceStore.handleWsEvent),
      // Flight recorder — receives ALL topics
      subscribe(WS_TOPIC.SESSION, activityStore.handleWsEvent),
      subscribe(WS_TOPIC.ROOM, activityStore.handleWsEvent),
      subscribe(WS_TOPIC.WORKFLOW, activityStore.handleWsEvent),
    ]
    return () => {
      unsubs.forEach((fn) => fn())
    }
  }, [subscribe])

  // Re-fetch stale data when events are missed (server lag notification)
  useEffect(() => {
    const handleEventsMissed = () => {
      void sessionStore.fetchAll()
      void workflowStore.fetchIfStale()
    }
    window.addEventListener('ws:events-missed', handleEventsMissed)
    return () => {
      window.removeEventListener('ws:events-missed', handleEventsMissed)
    }
  }, [])

  return null
}

export { WsStoreRouter }
