import { workflowExecutionStore } from './workflowExecutionStore'
import { WORKFLOW_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'

const getState = () => workflowExecutionStore.store.getState()

const makeMsg = (event: string, data: Record<string, unknown>, runId: string | null = 'run-1'): WsWireMessage => ({
  topic: 'workflow',
  event,
  ts: '2025-01-01T00:00:00Z',
  run_id: runId,
  user_id: null,
  data,
})

beforeEach(() => {
  workflowExecutionStore.reset()
})

describe('workflowExecutionStore', () => {
  describe('handleWsEvent', () => {
    it('STARTED initializes execution state', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 3 }),
      )

      const s = getState()
      expect(s.runId).toBe('run-1')
      expect(s.workflowId).toBe('w1')
      expect(s.isRunning).toBe(true)
      expect(s.totalSteps).toBe(3)
      expect(s.startedAt).toBe('2025-01-01T00:00:00Z')
      expect(s.error).toBeNull()
    })

    it('STEP_STARTED sets step to running', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 2 }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'Step One',
          agent_id: 'a1',
          execution_id: 'exec1',
        }),
      )

      const step = getState().stepStates['s1']
      expect(step).toBeDefined()
      expect(step.status).toBe('running')
      expect(step.startedAt).toBe('2025-01-01T00:00:00Z')
    })

    it('STEP_COMPLETED sets step to success with metrics', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, {
          workflow_id: 'w1', step_id: 's1', step_name: 'Step One', agent_id: null, execution_id: null,
        }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'Step One',
          agent_id: null,
          output: 'result text',
          input_tokens: 100,
          output_tokens: 50,
          duration_ms: 1500,
        }),
      )

      const step = getState().stepStates['s1']
      expect(step.status).toBe('success')
      expect(step.output).toBe('result text')
      expect(step.inputTokens).toBe(100)
      expect(step.outputTokens).toBe(50)
      expect(step.durationMs).toBe(1500)
      expect(step.completedAt).toBe('2025-01-01T00:00:00Z')
    })

    it('STEP_FAILED sets step to error', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_FAILED, {
          workflow_id: 'w1', step_id: 's1', step_name: 'Step One', error: 'timeout',
        }),
      )

      const step = getState().stepStates['s1']
      expect(step.status).toBe('error')
      expect(step.error).toBe('timeout')
    })

    it('STEP_PAUSED sets step to paused', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_PAUSED, {
          workflow_id: 'w1', step_id: 's1', step_name: 'Step One',
        }),
      )

      expect(getState().stepStates['s1'].status).toBe('paused')
    })

    it('FOR_EACH_PROGRESS updates progress', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.FOR_EACH_PROGRESS, {
          workflow_id: 'w1', step_id: 's1', step_name: 'Step One', completed: 3, total: 10,
        }),
      )

      const step = getState().stepStates['s1']
      expect(step.forEachProgress).toEqual({ completed: 3, total: 10 })
    })

    it('COMPLETED marks workflow as finished', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.COMPLETED, { workflow_id: 'w1', duration_ms: 5000 }),
      )

      const s = getState()
      expect(s.isRunning).toBe(false)
      expect(s.durationMs).toBe(5000)
      expect(s.completedAt).toBe('2025-01-01T00:00:00Z')
    })

    it('FAILED marks workflow as failed with error', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.FAILED, { workflow_id: 'w1', error: 'step explosion' }),
      )

      const s = getState()
      expect(s.isRunning).toBe(false)
      expect(s.error).toBe('step explosion')
    })

    it('RESUMED sets running and updates step', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 2 }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_PAUSED, { workflow_id: 'w1', step_id: 's1', step_name: 'Step One' }),
      )
      workflowExecutionStore.store.setState({ isRunning: false })

      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.RESUMED, { workflow_id: 'w1', step_id: 's1' }),
      )

      expect(getState().isRunning).toBe(true)
      expect(getState().stepStates['s1'].status).toBe('running')
    })

    it('ignores unknown events', () => {
      const before = getState()
      workflowExecutionStore.handleWsEvent(
        makeMsg('unknown_event', { foo: 'bar' }),
      )
      expect(getState()).toBe(before)
    })
  })

  describe('reset', () => {
    it('clears all execution state', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 3 }),
      )

      workflowExecutionStore.reset()

      const s = getState()
      expect(s.runId).toBeNull()
      expect(s.workflowId).toBeNull()
      expect(s.isRunning).toBe(false)
      expect(s.stepStates).toEqual({})
      expect(s.totalSteps).toBe(0)
    })
  })

  describe('selectors', () => {
    it('selectStepState returns step or undefined', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, {
          workflow_id: 'w1', step_id: 's1', step_name: 'Step One', agent_id: null, execution_id: null,
        }),
      )

      expect(workflowExecutionStore.selectStepState('s1')(getState())?.status).toBe('running')
      expect(workflowExecutionStore.selectStepState('missing')(getState())).toBeUndefined()
    })

    it('selectCompletedStepCount counts success steps', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, {
          workflow_id: 'w1', step_id: 's1', step_name: 'A', agent_id: null,
          output: null, input_tokens: null, output_tokens: null, duration_ms: null,
        }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, {
          workflow_id: 'w1', step_id: 's2', step_name: 'B', agent_id: null,
          output: null, input_tokens: null, output_tokens: null, duration_ms: null,
        }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_FAILED, {
          workflow_id: 'w1', step_id: 's3', step_name: 'C', error: 'fail',
        }),
      )

      expect(workflowExecutionStore.selectCompletedStepCount(getState())).toBe(2)
    })
  })
})
