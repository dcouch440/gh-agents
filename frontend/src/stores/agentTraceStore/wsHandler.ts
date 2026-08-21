import { WORKFLOW_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'
import type {
  DebugSystemPromptData,
  DebugUserMessageData,
  DebugAssistantMessageData,
  DebugToolCallData,
  DebugToolResultData,
} from '@/types/ws'
import type { AgentTrace, AgentTraceEvent } from './types'
import { store } from './_store'
import { setHydratedRun } from './hydrate'

const appendEvent = (agentExecutionId: string, agentName: string | null, stepId: string, event: AgentTraceEvent): void => {
  store.setState((s) => {
    const existing = s.traces[agentExecutionId]
    if (existing) {
      return {
        traces: {
          ...s.traces,
          [agentExecutionId]: { ...existing, events: [...existing.events, event] },
        },
      }
    }
    const trace: AgentTrace = {
      agentExecutionId,
      agentName,
      stepId,
      events: [event],
    }
    return {
      traces: { ...s.traces, [agentExecutionId]: trace },
      order: [...s.order, agentExecutionId],
    }
  })
}

const handleWsEvent = (msg: WsWireMessage): void => {
  try {
    switch (msg.event) {
      case WORKFLOW_EVENT.STARTED: {
        // Stamp the run that is actually starting, not just "clear everything".
        // `workflowLiveStore`'s poller calls `setHydratedRun(run.id)` on every
        // tick; if this left `hydratedRunId` at `null` instead, the next tick
        // would see `null -> run.id` as a run change and wipe out any trace
        // events that arrived in between.
        setHydratedRun(msg.run_id)
        break
      }
      case WORKFLOW_EVENT.DEBUG_SYSTEM_PROMPT: {
        const d = msg.data as DebugSystemPromptData
        appendEvent(d.agent_execution_id, d.agent_name, d.step_id, {
          type: 'system_prompt',
          content: d.content,
          ts: msg.ts,
        })
        break
      }
      case WORKFLOW_EVENT.DEBUG_USER_MESSAGE: {
        const d = msg.data as DebugUserMessageData
        appendEvent(d.agent_execution_id, d.agent_name, d.step_id, {
          type: 'user_message',
          content: d.content,
          ts: msg.ts,
        })
        break
      }
      case WORKFLOW_EVENT.DEBUG_ASSISTANT_MESSAGE: {
        const d = msg.data as DebugAssistantMessageData
        appendEvent(d.agent_execution_id, d.agent_name, d.step_id, {
          type: 'assistant_message',
          content: d.content,
          ts: msg.ts,
        })
        break
      }
      case WORKFLOW_EVENT.DEBUG_TOOL_CALL: {
        const d = msg.data as DebugToolCallData
        appendEvent(d.agent_execution_id, d.agent_name, d.step_id, {
          type: 'tool_call',
          toolName: d.tool_name,
          toolId: d.tool_id,
          input: d.input,
          ts: msg.ts,
        })
        break
      }
      case WORKFLOW_EVENT.DEBUG_TOOL_RESULT: {
        const d = msg.data as DebugToolResultData
        appendEvent(d.agent_execution_id, d.agent_name, d.step_id, {
          type: 'tool_result',
          toolName: d.tool_name,
          toolId: d.tool_id,
          result: d.result,
          ts: msg.ts,
        })
        break
      }
    }
  } catch (err) {
    console.error(`[agentTraceStore] WS handler error on "${msg.event}":`, err)
  }
}

export { handleWsEvent }
