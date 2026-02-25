// ============================================================================
// parseWsEvent — Single parse boundary: WsWireMessage → ActivityEvent | null
//
// This is the ONLY place where raw wire strings are matched and `as` casts
// on msg.data are performed. Everything downstream gets typed ActivityEvent.
// Returns null for unknown events (forward-compatible with new server events).
// ============================================================================

import { WS_TOPIC, WORKFLOW_EVENT, ROOM_EVENT, SESSION_EVENT } from '@/types/ws'
import { ACTIVITY } from '@/types/activity'
import type { WsWireMessage } from '@/types/ws'
import type {
  WorkflowStartedData,
  StepStartedData,
  StepCompletedData,
  StepFailedData,
  StepPausedData,
  ForEachProgressData,
  WorkflowCompletedData,
  WorkflowFailedData,
  WorkflowResumedData,
  SubWorkflowStartedData,
  SubWorkflowCompletedData,
  SubWorkflowStepProgressData,
  SpeakerStartData,
  SpeakerTokenData,
  SpeakerEndData,
  TurnCompleteData,
  SessionCompleteData,
  SessionCreatedData,
  SessionUpdatedData,
  SessionDeletedData,
  DispatchStartedData,
  DispatchProgressData,
  DispatchCompletedData,
  DispatchFailedData,
  DispatchCancelledData,
  DispatchStreamTokenData,
  DispatchStreamToolStartData,
  DispatchStreamToolEndData,
  DispatchStreamErrorData,
} from '@/types/ws'
import type { ActivityEvent } from '@/types/activity'

const parseWsEvent = (msg: WsWireMessage): ActivityEvent | null => {
  switch (msg.topic) {
    case WS_TOPIC.WORKFLOW:
      return parseWorkflowEvent(msg)
    case WS_TOPIC.ROOM:
      return parseRoomEvent(msg)
    case WS_TOPIC.SESSION:
      return parseSessionEvent(msg)
    default:
      return null
  }
}

// ── Workflow ─────────────────────────────────────────────────────────────────

const parseWorkflowEvent = (msg: WsWireMessage): ActivityEvent | null => {
  switch (msg.event) {
    case WORKFLOW_EVENT.STARTED: {
      const d = msg.data as WorkflowStartedData
      return { type: ACTIVITY.WORKFLOW_STARTED, workflowId: d.workflow_id, totalSteps: d.total_steps }
    }
    case WORKFLOW_EVENT.STEP_STARTED: {
      const d = msg.data as StepStartedData
      return {
        type: ACTIVITY.WORKFLOW_STEP_STARTED,
        workflowId: d.workflow_id,
        stepId: d.step_id,
        stepName: d.step_name,
        agentId: d.agent_id ?? null,
        executionId: d.execution_id ?? null,
      }
    }
    case WORKFLOW_EVENT.STEP_COMPLETED: {
      const d = msg.data as StepCompletedData
      return {
        type: ACTIVITY.WORKFLOW_STEP_COMPLETED,
        workflowId: d.workflow_id,
        stepId: d.step_id,
        stepName: d.step_name,
        agentId: d.agent_id ?? null,
        output: d.output ?? null,
        inputTokens: d.input_tokens ?? null,
        outputTokens: d.output_tokens ?? null,
        durationMs: d.duration_ms ?? null,
      }
    }
    case WORKFLOW_EVENT.STEP_FAILED: {
      const d = msg.data as StepFailedData
      return {
        type: ACTIVITY.WORKFLOW_STEP_FAILED,
        workflowId: d.workflow_id,
        stepId: d.step_id,
        stepName: d.step_name,
        error: d.error,
      }
    }
    case WORKFLOW_EVENT.STEP_PAUSED: {
      const d = msg.data as StepPausedData
      return {
        type: ACTIVITY.WORKFLOW_STEP_PAUSED,
        workflowId: d.workflow_id,
        stepId: d.step_id,
        stepName: d.step_name,
      }
    }
    case WORKFLOW_EVENT.FOR_EACH_PROGRESS: {
      const d = msg.data as ForEachProgressData
      return {
        type: ACTIVITY.WORKFLOW_FOR_EACH_PROGRESS,
        workflowId: d.workflow_id,
        stepId: d.step_id,
        stepName: d.step_name,
        completed: d.completed,
        total: d.total,
      }
    }
    case WORKFLOW_EVENT.COMPLETED: {
      const d = msg.data as WorkflowCompletedData
      return { type: ACTIVITY.WORKFLOW_COMPLETED, workflowId: d.workflow_id, durationMs: d.duration_ms ?? null }
    }
    case WORKFLOW_EVENT.FAILED: {
      const d = msg.data as WorkflowFailedData
      return { type: ACTIVITY.WORKFLOW_FAILED, workflowId: d.workflow_id, error: d.error }
    }
    case WORKFLOW_EVENT.RESUMED: {
      const d = msg.data as WorkflowResumedData
      return { type: ACTIVITY.WORKFLOW_RESUMED, workflowId: d.workflow_id, stepId: d.step_id }
    }
    case WORKFLOW_EVENT.SUB_WORKFLOW_STARTED: {
      const d = msg.data as SubWorkflowStartedData
      return {
        type: ACTIVITY.WORKFLOW_SUB_WORKFLOW_STARTED,
        workflowId: d.workflow_id,
        parentStepId: d.parent_step_id,
        childExecutionId: d.child_execution_id,
        totalSteps: d.total_steps,
      }
    }
    case WORKFLOW_EVENT.SUB_WORKFLOW_COMPLETED: {
      const d = msg.data as SubWorkflowCompletedData
      return {
        type: ACTIVITY.WORKFLOW_SUB_WORKFLOW_COMPLETED,
        workflowId: d.workflow_id,
        parentStepId: d.parent_step_id,
        childExecutionId: d.child_execution_id,
        status: d.status,
      }
    }
    case WORKFLOW_EVENT.SUB_WORKFLOW_STEP_PROGRESS: {
      const d = msg.data as SubWorkflowStepProgressData
      return {
        type: ACTIVITY.WORKFLOW_SUB_WORKFLOW_STEP_PROGRESS,
        workflowId: d.workflow_id,
        parentStepId: d.parent_step_id,
        childExecutionId: d.child_execution_id,
        childStepId: d.child_step_id,
        childStepName: d.child_step_name,
        status: d.status,
        inputTokens: d.input_tokens ?? null,
        outputTokens: d.output_tokens ?? null,
        durationMs: d.duration_ms ?? null,
        error: d.error ?? null,
      }
    }
    default:
      return null
  }
}

// ── Room ─────────────────────────────────────────────────────────────────────

const parseRoomEvent = (msg: WsWireMessage): ActivityEvent | null => {
  switch (msg.event) {
    case ROOM_EVENT.SPEAKER_START: {
      const d = msg.data as SpeakerStartData
      return {
        type: ACTIVITY.ROOM_SPEAKER_START,
        roomSessionId: d.room_session_id,
        agentId: d.agent_id,
        agentName: d.agent_name,
        speakerOrder: d.speaker_order,
        turnNumber: d.turn_number,
      }
    }
    case ROOM_EVENT.SPEAKER_TOKEN: {
      const d = msg.data as SpeakerTokenData
      return {
        type: ACTIVITY.ROOM_SPEAKER_TOKEN,
        roomSessionId: d.room_session_id,
        agentId: d.agent_id,
        agentName: d.agent_name,
        content: d.content,
        speakerOrder: d.speaker_order,
        turnNumber: d.turn_number,
      }
    }
    case ROOM_EVENT.SPEAKER_END: {
      const d = msg.data as SpeakerEndData
      return {
        type: ACTIVITY.ROOM_SPEAKER_END,
        roomSessionId: d.room_session_id,
        agentId: d.agent_id,
        agentName: d.agent_name,
        content: d.content,
        speakerOrder: d.speaker_order,
        turnNumber: d.turn_number,
      }
    }
    case ROOM_EVENT.TURN_COMPLETE: {
      const d = msg.data as TurnCompleteData
      return {
        type: ACTIVITY.ROOM_TURN_COMPLETE,
        roomSessionId: d.room_session_id,
        turnNumber: d.turn_number,
      }
    }
    case ROOM_EVENT.SESSION_COMPLETE: {
      const d = msg.data as SessionCompleteData
      return {
        type: ACTIVITY.ROOM_SESSION_COMPLETE,
        roomSessionId: d.room_session_id,
        turnNumber: d.turn_number,
      }
    }
    default:
      return null
  }
}

// ── Session ──────────────────────────────────────────────────────────────────

const parseSessionEvent = (msg: WsWireMessage): ActivityEvent | null => {
  switch (msg.event) {
    case SESSION_EVENT.CREATED: {
      const d = msg.data as SessionCreatedData
      return {
        type: ACTIVITY.SESSION_CREATED,
        sessionId: d.session_id,
        title: d.title,
        modeId: d.mode_id,
      }
    }
    case SESSION_EVENT.UPDATED: {
      const d = msg.data as SessionUpdatedData
      return {
        type: ACTIVITY.SESSION_UPDATED,
        sessionId: d.session_id,
        title: d.title ?? null,
        modeId: d.mode_id ?? null,
      }
    }
    case SESSION_EVENT.DELETED: {
      const d = msg.data as SessionDeletedData
      return { type: ACTIVITY.SESSION_DELETED, sessionId: d.session_id }
    }
    case SESSION_EVENT.DISPATCH_STARTED: {
      const d = msg.data as DispatchStartedData
      return { type: ACTIVITY.DISPATCH_STARTED, stepId: d.step_id, executionId: d.execution_id, instruction: d.instruction }
    }
    case SESSION_EVENT.DISPATCH_PROGRESS: {
      const d = msg.data as DispatchProgressData
      return { type: ACTIVITY.DISPATCH_PROGRESS, stepId: d.step_id, message: d.message }
    }
    case SESSION_EVENT.DISPATCH_COMPLETED: {
      const d = msg.data as DispatchCompletedData
      return { type: ACTIVITY.DISPATCH_COMPLETED, stepId: d.step_id, summary: d.summary }
    }
    case SESSION_EVENT.DISPATCH_FAILED: {
      const d = msg.data as DispatchFailedData
      return { type: ACTIVITY.DISPATCH_FAILED, stepId: d.step_id, error: d.error }
    }
    case SESSION_EVENT.DISPATCH_CANCELLED: {
      const d = msg.data as DispatchCancelledData
      return { type: ACTIVITY.DISPATCH_CANCELLED, stepId: d.step_id }
    }
    case SESSION_EVENT.DISPATCH_STREAM_TOKEN: {
      const d = msg.data as DispatchStreamTokenData
      return { type: ACTIVITY.DISPATCH_STREAM_TOKEN, stepId: d.step_id, content: d.content }
    }
    case SESSION_EVENT.DISPATCH_STREAM_TOOL_START: {
      const d = msg.data as DispatchStreamToolStartData
      return { type: ACTIVITY.DISPATCH_STREAM_TOOL_START, stepId: d.step_id, toolName: d.tool_name, toolId: d.tool_id }
    }
    case SESSION_EVENT.DISPATCH_STREAM_TOOL_END: {
      const d = msg.data as DispatchStreamToolEndData
      return { type: ACTIVITY.DISPATCH_STREAM_TOOL_END, stepId: d.step_id, toolName: d.tool_name, toolId: d.tool_id }
    }
    case SESSION_EVENT.DISPATCH_STREAM_ERROR: {
      const d = msg.data as DispatchStreamErrorData
      return { type: ACTIVITY.DISPATCH_STREAM_ERROR, stepId: d.step_id, error: d.error }
    }
    default:
      return null
  }
}

export { parseWsEvent }
