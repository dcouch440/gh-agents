// ============================================================================
// ActivityEvent — Discriminated union over all WebSocket broadcast events
//
// ACTIVITY constant is the single source of truth for event type strings.
// Type definitions reference the constants, so adding a variant requires
// updating exactly one place. Use ACTIVITY.* everywhere, never raw strings.
// ============================================================================

// ── Constants ────────────────────────────────────────────────────────────────

export const ACTIVITY = {
  // Workflow
  WORKFLOW_STARTED: 'workflow:started',
  WORKFLOW_STEP_STARTED: 'workflow:step_started',
  WORKFLOW_STEP_COMPLETED: 'workflow:step_completed',
  WORKFLOW_STEP_FAILED: 'workflow:step_failed',
  WORKFLOW_STEP_PAUSED: 'workflow:step_paused',
  WORKFLOW_FOR_EACH_PROGRESS: 'workflow:for_each_progress',
  WORKFLOW_COMPLETED: 'workflow:completed',
  WORKFLOW_FAILED: 'workflow:failed',
  WORKFLOW_RESUMED: 'workflow:resumed',
  // Sub-workflow
  WORKFLOW_SUB_WORKFLOW_STARTED: 'workflow:sub_workflow_started',
  WORKFLOW_SUB_WORKFLOW_COMPLETED: 'workflow:sub_workflow_completed',
  WORKFLOW_SUB_WORKFLOW_STEP_PROGRESS: 'workflow:sub_workflow_step_progress',
  // Room
  ROOM_SPEAKER_START: 'room:speaker_start',
  ROOM_SPEAKER_TOKEN: 'room:speaker_token',
  ROOM_SPEAKER_END: 'room:speaker_end',
  ROOM_TURN_COMPLETE: 'room:turn_complete',
  ROOM_SESSION_COMPLETE: 'room:session_complete',
  // Session
  SESSION_CREATED: 'session:created',
  SESSION_UPDATED: 'session:updated',
  SESSION_DELETED: 'session:deleted',
  // Dispatch
  DISPATCH_STARTED: 'dispatch:started',
  DISPATCH_PROGRESS: 'dispatch:progress',
  DISPATCH_COMPLETED: 'dispatch:completed',
  DISPATCH_FAILED: 'dispatch:failed',
  DISPATCH_CANCELLED: 'dispatch:cancelled',
  DISPATCH_STREAM_TOKEN: 'dispatch:stream_token',
  DISPATCH_STREAM_TOOL_START: 'dispatch:stream_tool_start',
  DISPATCH_STREAM_TOOL_END: 'dispatch:stream_tool_end',
  DISPATCH_STREAM_ERROR: 'dispatch:stream_error',
} as const

// ── Workflow variants ────────────────────────────────────────────────────────

type WorkflowStartedEvent = {
  type: typeof ACTIVITY.WORKFLOW_STARTED
  workflowId: string
  totalSteps: number
}

type WorkflowStepStartedEvent = {
  type: typeof ACTIVITY.WORKFLOW_STEP_STARTED
  workflowId: string
  stepId: string
  stepName: string
  agentId: string | null
  executionId: string | null
}

type WorkflowStepCompletedEvent = {
  type: typeof ACTIVITY.WORKFLOW_STEP_COMPLETED
  workflowId: string
  stepId: string
  stepName: string
  agentId: string | null
  output: string | null
  inputTokens: number | null
  outputTokens: number | null
  durationMs: number | null
}

type WorkflowStepFailedEvent = {
  type: typeof ACTIVITY.WORKFLOW_STEP_FAILED
  workflowId: string
  stepId: string
  stepName: string
  error: string
}

type WorkflowStepPausedEvent = {
  type: typeof ACTIVITY.WORKFLOW_STEP_PAUSED
  workflowId: string
  stepId: string
  stepName: string
}

type WorkflowForEachProgressEvent = {
  type: typeof ACTIVITY.WORKFLOW_FOR_EACH_PROGRESS
  workflowId: string
  stepId: string
  stepName: string
  completed: number
  total: number
}

type WorkflowCompletedEvent = {
  type: typeof ACTIVITY.WORKFLOW_COMPLETED
  workflowId: string
  durationMs: number | null
}

type WorkflowFailedEvent = {
  type: typeof ACTIVITY.WORKFLOW_FAILED
  workflowId: string
  error: string
}

type WorkflowResumedEvent = {
  type: typeof ACTIVITY.WORKFLOW_RESUMED
  workflowId: string
  stepId: string
}

// ── Sub-workflow variants ────────────────────────────────────────────────────

type WorkflowSubWorkflowStartedEvent = {
  type: typeof ACTIVITY.WORKFLOW_SUB_WORKFLOW_STARTED
  workflowId: string
  parentStepId: string
  childExecutionId: string
  totalSteps: number
}

type WorkflowSubWorkflowCompletedEvent = {
  type: typeof ACTIVITY.WORKFLOW_SUB_WORKFLOW_COMPLETED
  workflowId: string
  parentStepId: string
  childExecutionId: string
  status: string
}

type WorkflowSubWorkflowStepProgressEvent = {
  type: typeof ACTIVITY.WORKFLOW_SUB_WORKFLOW_STEP_PROGRESS
  workflowId: string
  parentStepId: string
  childExecutionId: string
  childStepId: string
  childStepName: string
  status: string
  inputTokens: number | null
  outputTokens: number | null
  durationMs: number | null
  error: string | null
}

// ── Room variants ────────────────────────────────────────────────────────────

type RoomSpeakerStartEvent = {
  type: typeof ACTIVITY.ROOM_SPEAKER_START
  roomSessionId: string
  agentId: string
  agentName: string
  speakerOrder: number
  turnNumber: number
}

type RoomSpeakerTokenEvent = {
  type: typeof ACTIVITY.ROOM_SPEAKER_TOKEN
  roomSessionId: string
  agentId: string
  agentName: string
  content: string
  speakerOrder: number
  turnNumber: number
}

type RoomSpeakerEndEvent = {
  type: typeof ACTIVITY.ROOM_SPEAKER_END
  roomSessionId: string
  agentId: string
  agentName: string
  content: string
  speakerOrder: number
  turnNumber: number
}

type RoomTurnCompleteEvent = {
  type: typeof ACTIVITY.ROOM_TURN_COMPLETE
  roomSessionId: string
  turnNumber: number
}

type RoomSessionCompleteEvent = {
  type: typeof ACTIVITY.ROOM_SESSION_COMPLETE
  roomSessionId: string
  turnNumber: number
}

// ── Session variants ─────────────────────────────────────────────────────────

type SessionCreatedEvent = {
  type: typeof ACTIVITY.SESSION_CREATED
  sessionId: string
  title: string
  modeId: string
}

type SessionUpdatedEvent = {
  type: typeof ACTIVITY.SESSION_UPDATED
  sessionId: string
  title: string | null
  modeId: string | null
}

type SessionDeletedEvent = {
  type: typeof ACTIVITY.SESSION_DELETED
  sessionId: string
}

// ── Dispatch variants ──────────────────────────────────────────────────────

type DispatchStartedEvent = {
  type: typeof ACTIVITY.DISPATCH_STARTED
  stepId: string
  executionId: string
  instruction: string
}

type DispatchProgressEvent = {
  type: typeof ACTIVITY.DISPATCH_PROGRESS
  stepId: string
  message: string
}

type DispatchCompletedEvent = {
  type: typeof ACTIVITY.DISPATCH_COMPLETED
  stepId: string
  summary: string
}

type DispatchFailedEvent = {
  type: typeof ACTIVITY.DISPATCH_FAILED
  stepId: string
  error: string
}

type DispatchCancelledEvent = {
  type: typeof ACTIVITY.DISPATCH_CANCELLED
  stepId: string
}

type DispatchStreamTokenEvent = {
  type: typeof ACTIVITY.DISPATCH_STREAM_TOKEN
  stepId: string
  content: string
}

type DispatchStreamToolStartEvent = {
  type: typeof ACTIVITY.DISPATCH_STREAM_TOOL_START
  stepId: string
  toolName: string
  toolId: string
}

type DispatchStreamToolEndEvent = {
  type: typeof ACTIVITY.DISPATCH_STREAM_TOOL_END
  stepId: string
  toolName: string
  toolId: string
}

type DispatchStreamErrorEvent = {
  type: typeof ACTIVITY.DISPATCH_STREAM_ERROR
  stepId: string
  error: string
}

// ── Union ────────────────────────────────────────────────────────────────────

type ActivityEvent =
  | WorkflowStartedEvent
  | WorkflowStepStartedEvent
  | WorkflowStepCompletedEvent
  | WorkflowStepFailedEvent
  | WorkflowStepPausedEvent
  | WorkflowForEachProgressEvent
  | WorkflowCompletedEvent
  | WorkflowFailedEvent
  | WorkflowResumedEvent
  | WorkflowSubWorkflowStartedEvent
  | WorkflowSubWorkflowCompletedEvent
  | WorkflowSubWorkflowStepProgressEvent
  | RoomSpeakerStartEvent
  | RoomSpeakerTokenEvent
  | RoomSpeakerEndEvent
  | RoomTurnCompleteEvent
  | RoomSessionCompleteEvent
  | SessionCreatedEvent
  | SessionUpdatedEvent
  | SessionDeletedEvent
  | DispatchStartedEvent
  | DispatchProgressEvent
  | DispatchCompletedEvent
  | DispatchFailedEvent
  | DispatchCancelledEvent
  | DispatchStreamTokenEvent
  | DispatchStreamToolStartEvent
  | DispatchStreamToolEndEvent
  | DispatchStreamErrorEvent

// ── Helpers ──────────────────────────────────────────────────────────────────

type ActivityTopic = 'workflow' | 'room' | 'session'

/**
 * Extract the variant from ActivityEvent whose `type` matches K.
 * Useful for typing handler functions for specific event kinds.
 */
type ActivityEventOf<K extends ActivityEvent['type']> = Extract<ActivityEvent, { type: K }>

// ── Exports ──────────────────────────────────────────────────────────────────

export type {
  ActivityEvent,
  ActivityEventOf,
  ActivityTopic,
  WorkflowStartedEvent,
  WorkflowStepStartedEvent,
  WorkflowStepCompletedEvent,
  WorkflowStepFailedEvent,
  WorkflowStepPausedEvent,
  WorkflowForEachProgressEvent,
  WorkflowCompletedEvent,
  WorkflowFailedEvent,
  WorkflowResumedEvent,
  WorkflowSubWorkflowStartedEvent,
  WorkflowSubWorkflowCompletedEvent,
  WorkflowSubWorkflowStepProgressEvent,
  RoomSpeakerStartEvent,
  RoomSpeakerTokenEvent,
  RoomSpeakerEndEvent,
  RoomTurnCompleteEvent,
  RoomSessionCompleteEvent,
  SessionCreatedEvent,
  SessionUpdatedEvent,
  SessionDeletedEvent,
  DispatchStartedEvent,
  DispatchProgressEvent,
  DispatchCompletedEvent,
  DispatchFailedEvent,
  DispatchCancelledEvent,
  DispatchStreamTokenEvent,
  DispatchStreamToolStartEvent,
  DispatchStreamToolEndEvent,
  DispatchStreamErrorEvent,
}
