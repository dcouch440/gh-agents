// ============================================================================
// WebSocket Event Types
// ============================================================================

export const WS_TOPIC = {
  WORKFLOW: 'workflow',
  ROOM: 'room',
  SESSION: 'session',
} as const

export type WsTopic = (typeof WS_TOPIC)[keyof typeof WS_TOPIC]

// Wire format: every server event follows this shape
export type WsWireMessage<T = Record<string, unknown>> = {
  topic: WsTopic
  event: string
  ts: string
  run_id: string | null
  user_id: string | null
  data: T
}

// Control message types
export const WS_CONTROL = {
  SUBSCRIBED: 'subscribed',
  ERROR: 'error',
  PONG: 'pong',
} as const

// Control messages (direct socket responses, not broadcast)
export type WsControlMessage =
  | { type: typeof WS_CONTROL.SUBSCRIBED; topics: WsTopic[] }
  | { type: typeof WS_CONTROL.ERROR; message: string }
  | { type: typeof WS_CONTROL.PONG; client_ts: string; server_ts: string }

// Client message types
export const WS_MSG = {
  SUBSCRIBE: 'subscribe',
  UNSUBSCRIBE: 'unsubscribe',
  SUBSCRIBE_RUN: 'subscribe_run',
  UNSUBSCRIBE_RUN: 'unsubscribe_run',
  PING: 'ping',
} as const

// Client messages (sent from client to server)
export type WsClientMessage =
  | { type: typeof WS_MSG.SUBSCRIBE; topics: WsTopic[] }
  | { type: typeof WS_MSG.UNSUBSCRIBE; topics: WsTopic[] }
  | { type: typeof WS_MSG.SUBSCRIBE_RUN; run_id: string }
  | { type: typeof WS_MSG.UNSUBSCRIBE_RUN; run_id: string }
  | { type: typeof WS_MSG.PING; ts: string }

// Connection status
export const WS_STATUS = {
  CONNECTING: 'connecting',
  CONNECTED: 'connected',
  DISCONNECTED: 'disconnected',
} as const

export type WsStatus = (typeof WS_STATUS)[keyof typeof WS_STATUS]

// ============================================================================
// Workflow Events
// ============================================================================

export const WORKFLOW_EVENT = {
  STARTED: 'started',
  STEP_STARTED: 'step_started',
  STEP_COMPLETED: 'step_completed',
  STEP_FAILED: 'step_failed',
  STEP_PAUSED: 'step_paused',
  FOR_EACH_PROGRESS: 'for_each_progress',
  COMPLETED: 'completed',
  FAILED: 'failed',
  RESUMED: 'resumed',
} as const

export type WorkflowStartedData = { workflow_id: string; total_steps: number }
export type StepStartedData = {
  workflow_id: string
  step_id: string
  step_name: string
  agent_id: string | null
  execution_id: string | null
}
export type StepCompletedData = {
  workflow_id: string
  step_id: string
  step_name: string
  agent_id: string | null
  output: string | null
  input_tokens: number | null
  output_tokens: number | null
  duration_ms: number | null
}
export type StepFailedData = { workflow_id: string; step_id: string; step_name: string; error: string }
export type StepPausedData = { workflow_id: string; step_id: string; step_name: string }
export type ForEachProgressData = { workflow_id: string; step_id: string; step_name: string; completed: number; total: number }
export type WorkflowCompletedData = { workflow_id: string; duration_ms: number | null }
export type WorkflowFailedData = { workflow_id: string; error: string }
export type WorkflowResumedData = { workflow_id: string; step_id: string }

// ============================================================================
// Room Events
// ============================================================================

export const ROOM_EVENT = {
  SPEAKER_START: 'speaker_start',
  SPEAKER_TOKEN: 'speaker_token',
  SPEAKER_END: 'speaker_end',
  TURN_COMPLETE: 'turn_complete',
  SESSION_COMPLETE: 'session_complete',
} as const

export type SpeakerStartData = { room_session_id: string; agent_id: string; agent_name: string; speaker_order: number; turn_number: number }
export type SpeakerTokenData = {
  room_session_id: string
  agent_id: string
  agent_name: string
  content: string
  speaker_order: number
  turn_number: number
}
export type SpeakerEndData = {
  room_session_id: string
  agent_id: string
  agent_name: string
  content: string
  speaker_order: number
  turn_number: number
}
export type TurnCompleteData = { room_session_id: string; turn_number: number }
export type SessionCompleteData = { room_session_id: string; turn_number: number }

// ============================================================================
// Session Events
// ============================================================================

export const SESSION_EVENT = {
  CREATED: 'created',
  UPDATED: 'updated',
  DELETED: 'deleted',
} as const

export type SessionCreatedData = { session_id: string; title: string; mode_id: string }
export type SessionUpdatedData = { session_id: string; title: string | null; mode_id: string | null }
export type SessionDeletedData = { session_id: string }

// ============================================================================
// Handler Type
// ============================================================================

export type WsEventHandler = (message: WsWireMessage) => void
