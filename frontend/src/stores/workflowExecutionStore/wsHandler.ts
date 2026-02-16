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
  SubWorkflowStartedData,
  SubWorkflowCompletedData,
  SubWorkflowStepProgressData,
} from '@/types/ws'
import type { StepTimelineEvent, ChildStepState } from './types'
import { store, updateStep } from './_store'
import { fetchRuns } from './history'

const appendEvent = (log: StepTimelineEvent[], event: StepTimelineEvent): StepTimelineEvent[] => [...log, event]

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
        break
      }
      case WORKFLOW_EVENT.STEP_COMPLETED: {
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
      case WORKFLOW_EVENT.RESUMED: {
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
      case WORKFLOW_EVENT.SUB_WORKFLOW_STARTED: {
        const d = msg.data as SubWorkflowStartedData
        store.setState((s) => ({
          stepStates: updateStep(s.stepStates, d.parent_step_id, {
            subWorkflowProgress: {
              childExecutionId: d.child_execution_id,
              totalSteps: d.total_steps,
              completedSteps: 0,
              status: 'running',
              childSteps: [],
            },
          }),
        }))
        break
      }
      case WORKFLOW_EVENT.SUB_WORKFLOW_STEP_PROGRESS: {
        const d = msg.data as SubWorkflowStepProgressData
        store.setState((s) => {
          const parentStep = s.stepStates[d.parent_step_id]
          if (!parentStep?.subWorkflowProgress) return {}

          const prev = parentStep.subWorkflowProgress
          const existingIdx = prev.childSteps.findIndex((cs) => cs.childStepId === d.child_step_id)

          const childStatus: ChildStepState['status'] =
            d.status === 'completed' ? 'success'
              : d.status === 'failed' ? 'error'
                : 'running'

          const updatedChild: ChildStepState = {
            childStepId: d.child_step_id,
            childStepName: d.child_step_name,
            status: childStatus,
            inputTokens: d.input_tokens ?? null,
            outputTokens: d.output_tokens ?? null,
            durationMs: d.duration_ms ?? null,
            error: d.error ?? null,
          }

          const nextChildren = existingIdx >= 0
            ? prev.childSteps.map((cs, i) => (i === existingIdx ? updatedChild : cs))
            : [...prev.childSteps, updatedChild]

          const isTerminal = childStatus === 'success' || childStatus === 'error'
          const wasRunning = existingIdx >= 0 && prev.childSteps[existingIdx].status === 'running'
          const completedDelta = isTerminal && (existingIdx < 0 || wasRunning) ? 1 : 0

          return {
            stepStates: updateStep(s.stepStates, d.parent_step_id, {
              subWorkflowProgress: {
                ...prev,
                completedSteps: prev.completedSteps + completedDelta,
                childSteps: nextChildren,
              },
            }),
          }
        })
        break
      }
      case WORKFLOW_EVENT.SUB_WORKFLOW_COMPLETED: {
        const d = msg.data as SubWorkflowCompletedData
        store.setState((s) => {
          const parentStep = s.stepStates[d.parent_step_id]
          if (!parentStep?.subWorkflowProgress) return {}

          return {
            stepStates: updateStep(s.stepStates, d.parent_step_id, {
              subWorkflowProgress: {
                ...parentStep.subWorkflowProgress,
                status: d.status === 'completed' ? 'completed' : 'failed',
              },
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
