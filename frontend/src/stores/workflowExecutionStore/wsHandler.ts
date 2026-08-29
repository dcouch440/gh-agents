import { WORKFLOW_EVENT } from '@/types/ws'
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
  WorkforceAgentProgressData,
  WorkforceDesignerProgressData,
} from '@/types/ws'
import type { StepTimelineEvent } from './types'
import { store, updateStep } from './_store'
import { fetchRuns } from './history'
import { setActiveRun } from './hydrate'
import { sidebarStore } from '../sidebarStore'

const appendEvent = (log: StepTimelineEvent[], event: StepTimelineEvent): StepTimelineEvent[] => [...log, event]

/**
 * Swap the overlay when an event belongs to a run we are not currently showing.
 *
 * Covers the gap where a refresh lands between two runs and we never see the
 * new run's `started` event: without this, run B's step events would merge into
 * run A's results. Only the client-side view buffer is affected.
 */
const ensureRunOverlay = (msg: WsWireMessage): void => {
  const s = store.getState()
  if (msg.run_id === null) return
  if (s.runId === msg.run_id) return
  setActiveRun(msg.run_id, s.workflowId)
}

const handleWsEvent = (msg: WsWireMessage): void => {
  try {
    switch (msg.event) {
      case WORKFLOW_EVENT.STARTED: {
        const d = msg.data as WorkflowStartedData
        store.setState({
          runId: msg.run_id,
          workflowId: d.workflow_id,
          isRunning: true,
          stepStates: {},
          eventLog: [],
          totalSteps: d.total_steps,
          completedStepCount: 0,
          durationMs: null,
          error: null,
          startedAt: msg.ts,
          completedAt: null,
          viewMode: 'live',
          selectedHistoricalRunId: null,
          historicalRun: null,
        })
        break
      }
      case WORKFLOW_EVENT.STEP_STARTED: {
        ensureRunOverlay(msg)
        const d = msg.data as StepStartedData
        store.setState((s) => ({
          stepStates: updateStep(s.stepStates, d.step_id, {
            status: 'running',
            stepName: d.step_name,
            agentId: d.agent_id ?? null,
            executionId: d.execution_id ?? null,
            startedAt: msg.ts,
          }),
          eventLog: appendEvent(s.eventLog, { stepId: d.step_id, stepName: d.step_name, eventType: 'started', ts: msg.ts }),
        }))
        sidebarStore.expandStep(d.step_id)
        break
      }
      case WORKFLOW_EVENT.STEP_COMPLETED: {
        ensureRunOverlay(msg)
        const d = msg.data as StepCompletedData
        store.setState((s) => ({
          completedStepCount: s.completedStepCount + 1,
          stepStates: updateStep(s.stepStates, d.step_id, {
            status: 'success',
            stepName: d.step_name,
            output: d.output ?? null,
            inputTokens: d.input_tokens ?? null,
            outputTokens: d.output_tokens ?? null,
            durationMs: d.duration_ms ?? null,
            completedAt: msg.ts,
          }),
          eventLog: appendEvent(s.eventLog, { stepId: d.step_id, stepName: d.step_name, eventType: 'completed', ts: msg.ts }),
        }))
        break
      }
      case WORKFLOW_EVENT.STEP_FAILED: {
        ensureRunOverlay(msg)
        const d = msg.data as StepFailedData
        store.setState((s) => ({
          stepStates: updateStep(s.stepStates, d.step_id, {
            status: 'error',
            stepName: d.step_name,
            error: d.error,
            completedAt: msg.ts,
          }),
          eventLog: appendEvent(s.eventLog, { stepId: d.step_id, stepName: d.step_name, eventType: 'failed', ts: msg.ts }),
        }))
        break
      }
      case WORKFLOW_EVENT.STEP_PAUSED: {
        ensureRunOverlay(msg)
        const d = msg.data as StepPausedData
        store.setState((s) => ({
          stepStates: updateStep(s.stepStates, d.step_id, {
            status: 'paused',
            stepName: d.step_name,
          }),
          eventLog: appendEvent(s.eventLog, { stepId: d.step_id, stepName: d.step_name, eventType: 'paused', ts: msg.ts }),
        }))
        break
      }
      case WORKFLOW_EVENT.FOR_EACH_PROGRESS: {
        ensureRunOverlay(msg)
        const d = msg.data as ForEachProgressData
        store.setState((s) => ({
          stepStates: updateStep(s.stepStates, d.step_id, {
            forEachProgress: { completed: d.completed, total: d.total },
          }),
        }))
        break
      }
      case WORKFLOW_EVENT.COMPLETED: {
        const d = msg.data as WorkflowCompletedData
        const currentWorkflowId = store.getState().workflowId
        store.setState({
          isRunning: false,
          durationMs: d.duration_ms ?? null,
          completedAt: msg.ts,
        })
        if (currentWorkflowId) void fetchRuns(currentWorkflowId)
        break
      }
      case WORKFLOW_EVENT.FAILED: {
        const d = msg.data as WorkflowFailedData
        const currentWorkflowId = store.getState().workflowId
        store.setState({
          isRunning: false,
          error: d.error,
          completedAt: msg.ts,
        })
        if (currentWorkflowId) void fetchRuns(currentWorkflowId)
        break
      }
      case WORKFLOW_EVENT.CANCELLED: {
        // User-initiated, not a failure — leave `error` untouched so the
        // status resolves back to 'idle' rather than 'error'.
        const currentWorkflowId = store.getState().workflowId
        store.setState({
          isRunning: false,
          completedAt: msg.ts,
        })
        if (currentWorkflowId) void fetchRuns(currentWorkflowId)
        break
      }
      case WORKFLOW_EVENT.RESUMED: {
        ensureRunOverlay(msg)
        const d = msg.data as WorkflowResumedData
        store.setState((s) => ({
          isRunning: true,
          stepStates: updateStep(s.stepStates, d.step_id, {
            status: 'running',
            startedAt: msg.ts,
          }),
          eventLog: appendEvent(s.eventLog, { stepId: d.step_id, stepName: s.stepStates[d.step_id]?.stepName ?? null, eventType: 'resumed', ts: msg.ts }),
        }))
        break
      }
      case WORKFLOW_EVENT.WORKFORCE_DESIGNER_PROGRESS: {
        const d = msg.data as WorkforceDesignerProgressData
        store.setState((s) => {
          if (!s.isRunning) return {} // Ignore during dispatch/design phase — no execution active
          return {
            stepStates: updateStep(s.stepStates, d.step_id, {
              status: d.status === 'started' ? 'running' : d.status === 'completed' ? 'running' : 'error',
            }),
          }
        })
        break
      }
      case WORKFLOW_EVENT.WORKFORCE_AGENT_PROGRESS: {
        const d = msg.data as WorkforceAgentProgressData
        store.setState((s) => {
          if (!s.isRunning) return {} // Ignore during dispatch/design phase — no execution active
          return {
            stepStates: updateStep(s.stepStates, d.step_id, {
              forEachProgress: { completed: d.agent_index + (d.status === 'completed' ? 1 : 0), total: d.total_agents },
            }),
          }
        })
        break
      }
    }
  } catch (err) {
    console.error(`[workflowExecutionStore] WS handler error on "${msg.event}":`, err)
  }
}

export { handleWsEvent }
