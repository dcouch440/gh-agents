// ============================================================================
// wsConnectionStore — Module-level store for WebSocket connection state
// ============================================================================

import { createStore } from './lib'
import type { WsStatus } from '@/types/ws'
import { WS_STATUS } from '@/types/ws'

type WsConnectionState = {
  status: WsStatus
  latency: number | null
}

const wsConnectionStore = createStore<WsConnectionState>(() => ({
  status: WS_STATUS.DISCONNECTED,
  latency: null,
}))

export { wsConnectionStore }
export type { WsConnectionState }
