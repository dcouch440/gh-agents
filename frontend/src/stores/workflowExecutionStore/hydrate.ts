import { api } from '@/api'
import type { RunStepResult, ChildStepResult, WorkshopStepSummary } from '@/types'
import type { StepExecutionState, StepExecutionStatus, StepTimelineEvent, ChildStepState, SubWorkflowProgress } from './types'
import { store } from './_store'

const mapApiStatusToStoreStatus = (apiStatus: string): StepExecutionStatus => {
  switch (apiStatus) {
    case 'completed': return 'success'
    case 'failed': return 'error'
    case 'skipped': return 'skipped'
    case 'running': return 'running'
    case 'pending': return 'pending'
    case 'paused': return 'paused'
    default: return 'idle'
  }
}

const mapChildStatus = (status: string): ChildStepState['status'] => {
  if (status === 'completed') return 'success'
  if (status === 'failed') return 'error'
  return 'running'
}

const buildSubWorkflowProgress = (step: RunStepResult): SubWorkflowProgress | null => {
  if (step.child_execution_id === null || !step.child_steps || step.child_steps.length === 0) return null

  const terminalCount = step.child_steps.filter(
    (cs) => cs.status === 'completed' || cs.status === 'failed',
  ).length

  const progressStatus: SubWorkflowProgress['status'] =
    step.status === 'completed' ? 'completed'
      : step.status === 'failed' ? 'failed'
        : 'running'

  return {
    childExecutionId: step.child_execution_id,
    totalSteps: step.child_steps.length,
    completedSteps: terminalCount,
    status: progressStatus,
    childSteps: step.child_steps.map((cs: ChildStepResult) => ({
      childStepId: cs.step_name ?? 'unknown',
      childStepName: cs.step_name ?? 'unknown',
      status: mapChildStatus(cs.status),
      inputTokens: cs.input_tokens,
      outputTokens: cs.output_tokens,
      durationMs: cs.duration_ms,
      error: cs.error,
    })),
  }
}

const mapRunStepToStepState = (step: RunStepResult): StepExecutionState => ({
  status: mapApiStatusToStoreStatus(step.status),
  stepName: step.step_name,
  agentId: null,
  executionId: step.execution_id,
  output: step.output,
  error: step.error,
  inputTokens: step.input_tokens,
  outputTokens: step.output_tokens,
  durationMs: step.duration_ms,
  forEachProgress: null,
  subWorkflowProgress: buildSubWorkflowProgress(step),
  startedAt: step.started_at,
  completedAt: step.completed_at,
})

const buildEventLog = (steps: RunStepResult[]): StepTimelineEvent[] => {
  const events: StepTimelineEvent[] = []
  for (const step of steps) {
    if (step.started_at) {
      events.push({ stepId: step.step_id, stepName: step.step_name, eventType: 'started', ts: step.started_at })
    }
    if (step.status === 'completed' && step.completed_at) {
      events.push({ stepId: step.step_id, stepName: step.step_name, eventType: 'completed', ts: step.completed_at })
    } else if (step.status === 'failed' && step.completed_at) {
      events.push({ stepId: step.step_id, stepName: step.step_name, eventType: 'failed', ts: step.completed_at })
    }
  }
  events.sort((a, b) => a.ts.localeCompare(b.ts))
  return events
}

const hydrateLatestRun = async (workflowId: string): Promise<void> => {
  try {
    const runs = await api.workflows.listExecutions(workflowId)
    store.setState({ runs })

    if (runs.length === 0) return

    const latestRun = runs[0]

    // Race guard: if a WS event already populated live state, skip hydration
    if (store.getState().runId !== null) return

    const detail = await api.workflows.getRunDetail(workflowId, latestRun.id)

    // Race guard (post-fetch): re-check after async gap
    if (store.getState().runId !== null) return

    const stepStates: Record<string, StepExecutionState> = {}
    for (const step of detail.steps) {
      stepStates[step.step_id] = mapRunStepToStepState(step)
    }

    const eventLog = buildEventLog(detail.steps)

    const completedStepCount = detail.steps.filter(
      (s) => s.status === 'completed' || s.status === 'failed' || s.status === 'skipped',
    ).length

    store.setState({
      runId: latestRun.id,
      workflowId: latestRun.workflow_id,
      isRunning: latestRun.status === 'running',
      stepStates,
      eventLog,
      totalSteps: detail.steps.length,
      completedStepCount,
      durationMs: detail.duration_ms,
      error: latestRun.error,
      startedAt: latestRun.started_at,
      completedAt: latestRun.completed_at,
      viewMode: 'live',
    })
  } catch (err) {
    console.error('[workflowExecutionStore] hydration error:', err)
  }
}

const mapWorkshopStepToStepState = (step: WorkshopStepSummary, runId: string): StepExecutionState => ({
  status: mapApiStatusToStoreStatus(step.status),
  stepName: null,
  agentId: null,
  executionId: runId,
  output: step.output !== null ? JSON.stringify(step.output) : null,
  error: null,
  inputTokens: null,
  outputTokens: null,
  durationMs: null,
  forEachProgress: null,
  subWorkflowProgress: null,
  startedAt: null,
  completedAt: null,
})

const hydrateWorkshop = async (workflowId: string): Promise<void> => {
  try {
    const workshop = await api.workflows.getWorkshopStatus(workflowId)

    if (workshop.completed_steps.length === 0) return

    // Don't overwrite a live running execution
    if (store.getState().isRunning) return

    const stepStates: Record<string, StepExecutionState> = {}
    for (const step of workshop.completed_steps) {
      stepStates[step.step_id] = mapWorkshopStepToStepState(step, workshop.run_id)
    }

    store.setState({
      runId: workshop.run_id,
      workflowId: workshop.workflow_id,
      isRunning: false,
      stepStates,
      totalSteps: workshop.completed_steps.length + workshop.next_executable_steps.length,
      completedStepCount: workshop.completed_steps.length,
      viewMode: 'live',
    })
  } catch {
    // Workshop may not exist yet — no-op
  }
}

export { hydrateLatestRun, hydrateWorkshop, mapApiStatusToStoreStatus, mapRunStepToStepState, buildSubWorkflowProgress, buildEventLog }
