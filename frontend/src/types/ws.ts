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
  seq: number | null
  data: T
}

// Control message types
export const WS_CONTROL = {
  SUBSCRIBED: 'subscribed',
  ERROR: 'error',
  PONG: 'pong',
  EVENTS_MISSED: 'events_missed',
} as const

// Control messages (direct socket responses, not broadcast)
export type WsControlMessage =
  | { type: typeof WS_CONTROL.SUBSCRIBED; topics: WsTopic[] }
  | { type: typeof WS_CONTROL.ERROR; message: string }
  | { type: typeof WS_CONTROL.PONG; client_ts: string; server_ts: string }
  | { type: typeof WS_CONTROL.EVENTS_MISSED; missed_count: number; message: string }

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
  STEP_CONFIG_UPDATED: 'step_config_updated',
  ROSTER_CHANGED: 'roster_changed',
  ROOM_MEMBERS_CHANGED: 'room_members_changed',
  PLAN_UPDATED: 'plan_updated',
  CONSISTENCY_ISSUES: 'consistency_issues',
  SUB_WORKFLOW_STARTED: 'sub_workflow_started',
  SUB_WORKFLOW_COMPLETED: 'sub_workflow_completed',
  SUB_WORKFLOW_STEP_PROGRESS: 'sub_workflow_step_progress',
  // Workforce high-level progress (backend already emits these)
  WORKFORCE_DESIGNER_PROGRESS: 'workforce_designer_progress',
  WORKFORCE_AGENT_PROGRESS: 'workforce_agent_progress',
  // Generic step streaming (token-level events from any execution source)
  STEP_STREAM_TOKEN: 'step_stream_token',
  STEP_STREAM_TOOL_START: 'step_stream_tool_start',
  STEP_STREAM_TOOL_END: 'step_stream_tool_end',
  STEP_STREAM_ERROR: 'step_stream_error',
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
export type StepConfigUpdatedData = { workflow_id: string; step_id: string }
export type RosterChangedData = { workflow_id: string; step_id: string }
export type RoomMembersChangedData = { workflow_id: string; step_id: string }
export type PlanUpdatedData = { workflow_id: string; step_id: string; content: string }
export type ConsistencyIssue = {
  step_id: string
  step_name: string
  description: string
  severity: string
  deleted_item_name: string
  deleted_item_type: string
}
export type ConsistencyIssuesData = { workflow_id: string; issues: ConsistencyIssue[] }
export type SubWorkflowStartedData = { workflow_id: string; parent_step_id: string; child_execution_id: string; total_steps: number }
export type SubWorkflowCompletedData = { workflow_id: string; parent_step_id: string; child_execution_id: string; status: string }
export type SubWorkflowStepProgressData = {
  workflow_id: string; parent_step_id: string; child_execution_id: string
  child_step_id: string; child_step_name: string; status: string
  input_tokens: number | null; output_tokens: number | null; duration_ms: number | null; error: string | null
}

// Workforce high-level progress
export type WorkforceDesignerProgressData = { workflow_id: string; step_id: string; status: string }
export type WorkforceAgentProgressData = {
  workflow_id: string; step_id: string
  agent_name: string; roster_agent_id: string
  agent_index: number; total_agents: number; status: string
}

// Generic step streaming
export type StepStreamTokenData = { workflow_id: string; step_id: string; source_id: string; source_name: string; content: string }
export type StepStreamToolStartData = { workflow_id: string; step_id: string; source_id: string; source_name: string; tool_name: string; tool_id: string }
export type StepStreamToolEndData = { workflow_id: string; step_id: string; source_id: string; source_name: string; tool_name: string; tool_id: string }
export type StepStreamErrorData = { workflow_id: string; step_id: string; source_id: string; source_name: string; error: string }

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
  AGENT_MESSAGE: 'agent_message',
  DISPATCH_STARTED: 'dispatch_started',
  DISPATCH_PROGRESS: 'dispatch_progress',
  DISPATCH_COMPLETED: 'dispatch_completed',
  DISPATCH_FAILED: 'dispatch_failed',
  DISPATCH_CANCELLED: 'dispatch_cancelled',
  // Dispatch streaming events
  DISPATCH_STREAM_TOKEN: 'dispatch_stream_token',
  DISPATCH_STREAM_TOOL_START: 'dispatch_stream_tool_start',
  DISPATCH_STREAM_TOOL_END: 'dispatch_stream_tool_end',
  DISPATCH_STREAM_ERROR: 'dispatch_stream_error',
} as const

export type SessionCreatedData = { session_id: string; title: string; mode_id: string }
export type SessionUpdatedData = { session_id: string; title: string | null; mode_id: string | null }
export type SessionDeletedData = { session_id: string }
export type AgentMessageData = {
  session_id: string
  message_id: string
  from_agent: string
  message_type: string
  content_preview: string
}

// Dispatch events
export type DispatchStartedData = { session_id: string; execution_id: string; step_id: string; instruction: string }
export type DispatchProgressData = { session_id: string; execution_id: string; step_id: string; message: string }
export type DispatchCompletedData = { session_id: string; execution_id: string; step_id: string; summary: string; question: string | null }
export type DispatchFailedData = { session_id: string; execution_id: string; step_id: string; error: string }
export type DispatchCancelledData = { session_id: string; execution_id: string; step_id: string }

// Dispatch streaming events
export type DispatchStreamTokenData = {
  session_id: string; execution_id: string; step_id: string; content: string
}
export type DispatchStreamToolStartData = {
  session_id: string; execution_id: string; step_id: string
  tool_name: string; tool_id: string; input: Record<string, unknown>
}
export type DispatchStreamToolEndData = {
  session_id: string; execution_id: string; step_id: string
  tool_name: string; tool_id: string; result: unknown
}
export type DispatchStreamErrorData = {
  session_id: string; execution_id: string; step_id: string; error: string
}

// ============================================================================
// Handler Type
// ============================================================================

export type WsEventHandler = (message: WsWireMessage) => void
