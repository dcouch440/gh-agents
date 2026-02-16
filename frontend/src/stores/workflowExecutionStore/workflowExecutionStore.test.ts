import { workflowExecutionStore } from '.'
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
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 3 }))

      const s = getState()
      expect(s.runId).toBe('run-1')
      expect(s.workflowId).toBe('w1')
      expect(s.isRunning).toBe(true)
      expect(s.totalSteps).toBe(3)
      expect(s.startedAt).toBe('2025-01-01T00:00:00Z')
      expect(s.error).toBeNull()
    })

    it('STEP_STARTED sets step to running', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 2 }))
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
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'Step One',
          agent_id: null,
          execution_id: null,
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
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'Step One',
          error: 'timeout',
        }),
      )

      const step = getState().stepStates['s1']
      expect(step.status).toBe('error')
      expect(step.error).toBe('timeout')
    })

    it('STEP_PAUSED sets step to paused', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_PAUSED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'Step One',
        }),
      )

      expect(getState().stepStates['s1'].status).toBe('paused')
    })

    it('FOR_EACH_PROGRESS updates progress', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.FOR_EACH_PROGRESS, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'Step One',
          completed: 3,
          total: 10,
        }),
      )

      const step = getState().stepStates['s1']
      expect(step.forEachProgress).toEqual({ completed: 3, total: 10 })
    })

    it('COMPLETED marks workflow as finished', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.COMPLETED, { workflow_id: 'w1', duration_ms: 5000 }))

      const s = getState()
      expect(s.isRunning).toBe(false)
      expect(s.durationMs).toBe(5000)
      expect(s.completedAt).toBe('2025-01-01T00:00:00Z')
    })

    it('FAILED marks workflow as failed with error', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.FAILED, { workflow_id: 'w1', error: 'step explosion' }))

      const s = getState()
      expect(s.isRunning).toBe(false)
      expect(s.error).toBe('step explosion')
    })

    it('RESUMED sets running and updates step', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 2 }))
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STEP_PAUSED, { workflow_id: 'w1', step_id: 's1', step_name: 'Step One' }))
      workflowExecutionStore.store.setState({ isRunning: false })

      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.RESUMED, { workflow_id: 'w1', step_id: 's1' }))

      expect(getState().isRunning).toBe(true)
      expect(getState().stepStates['s1'].status).toBe('running')
    })

    it('ignores unknown events', () => {
      const before = getState()
      workflowExecutionStore.handleWsEvent(makeMsg('unknown_event', { foo: 'bar' }))
      expect(getState()).toBe(before)
    })
  })

  describe('eventLog', () => {
    it('STARTED clears eventLog', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 's1', step_name: 'A', agent_id: null, execution_id: null }),
      )
      expect(getState().eventLog).toHaveLength(1)

      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 2 }))
      expect(getState().eventLog).toEqual([])
    })

    it('STEP_STARTED appends to eventLog', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 2 }))
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 's1', step_name: 'Step A', agent_id: 'a1', execution_id: 'e1' }),
      )

      const log = getState().eventLog
      expect(log).toHaveLength(1)
      expect(log[0]).toEqual({ stepId: 's1', stepName: 'Step A', eventType: 'started', ts: '2025-01-01T00:00:00Z' })
    })

    it('STEP_COMPLETED appends to eventLog', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'Step A',
          agent_id: null,
          output: 'done',
          input_tokens: 10,
          output_tokens: 5,
          duration_ms: 100,
        }),
      )

      const log = getState().eventLog
      expect(log).toHaveLength(1)
      expect(log[0].eventType).toBe('completed')
    })

    it('STEP_FAILED appends to eventLog', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_FAILED, { workflow_id: 'w1', step_id: 's1', step_name: 'Step A', error: 'boom' }),
      )

      expect(getState().eventLog).toHaveLength(1)
      expect(getState().eventLog[0].eventType).toBe('failed')
    })

    it('STEP_PAUSED appends to eventLog', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STEP_PAUSED, { workflow_id: 'w1', step_id: 's1', step_name: 'Step A' }))

      expect(getState().eventLog).toHaveLength(1)
      expect(getState().eventLog[0].eventType).toBe('paused')
    })

    it('RESUMED appends to eventLog', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STEP_PAUSED, { workflow_id: 'w1', step_id: 's1', step_name: 'Step A' }))
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.RESUMED, { workflow_id: 'w1', step_id: 's1' }))

      const log = getState().eventLog
      expect(log).toHaveLength(2) // paused + resumed (STARTED clears log)
      expect(log[1].eventType).toBe('resumed')
      expect(log[1].stepName).toBe('Step A') // uses name from prior step state
    })

    it('maintains chronological order across events', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 2 }))
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 's1', step_name: 'A', agent_id: null, execution_id: null }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'A',
          agent_id: null,
          output: 'ok',
          input_tokens: 1,
          output_tokens: 1,
          duration_ms: 10,
        }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 's2', step_name: 'B', agent_id: null, execution_id: null }),
      )

      const log = getState().eventLog
      expect(log).toHaveLength(3)
      expect(log.map((e) => e.eventType)).toEqual(['started', 'completed', 'started'])
      expect(log.map((e) => e.stepId)).toEqual(['s1', 's1', 's2'])
    })
  })

  describe('step metadata capture', () => {
    it('STEP_STARTED captures stepName, agentId, executionId', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'Analyze Data',
          agent_id: 'agent-99',
          execution_id: 'exec-42',
        }),
      )

      const step = getState().stepStates['s1']
      expect(step.stepName).toBe('Analyze Data')
      expect(step.agentId).toBe('agent-99')
      expect(step.executionId).toBe('exec-42')
    })

    it('STEP_COMPLETED captures stepName', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'Generate Report',
          agent_id: null,
          output: 'report',
          input_tokens: 10,
          output_tokens: 5,
          duration_ms: 200,
        }),
      )

      expect(getState().stepStates['s1'].stepName).toBe('Generate Report')
    })

    it('STEP_FAILED captures stepName', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_FAILED, { workflow_id: 'w1', step_id: 's1', step_name: 'Parse Input', error: 'bad json' }),
      )

      expect(getState().stepStates['s1'].stepName).toBe('Parse Input')
    })

    it('STEP_PAUSED captures stepName', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_PAUSED, { workflow_id: 'w1', step_id: 's1', step_name: 'Human Review' }),
      )

      expect(getState().stepStates['s1'].stepName).toBe('Human Review')
    })
  })

  describe('reset', () => {
    it('clears all execution state including eventLog', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 3 }))
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 's1', step_name: 'A', agent_id: null, execution_id: null }),
      )

      workflowExecutionStore.reset()

      const s = getState()
      expect(s.runId).toBeNull()
      expect(s.workflowId).toBeNull()
      expect(s.isRunning).toBe(false)
      expect(s.stepStates).toEqual({})
      expect(s.totalSteps).toBe(0)
      expect(s.eventLog).toEqual([])
    })
  })

  describe('history actions', () => {
    it('viewHistoricalRun sets history mode and selects run', () => {
      workflowExecutionStore.store.setState({
        runs: [
          {
            id: 'run-a',
            workflow_id: 'w1',
            status: 'completed',
            started_at: '2025-01-01T00:00:00Z',
            completed_at: '2025-01-01T00:01:00Z',
            outputs: null,
            error: null,
          },
          {
            id: 'run-b',
            workflow_id: 'w1',
            status: 'failed',
            started_at: '2025-01-02T00:00:00Z',
            completed_at: '2025-01-02T00:01:00Z',
            outputs: null,
            error: 'oops',
          },
        ],
      })

      workflowExecutionStore.viewHistoricalRun('run-b')

      const s = getState()
      expect(s.viewMode).toBe('history')
      expect(s.selectedHistoricalRunId).toBe('run-b')
      expect(s.historicalRun?.id).toBe('run-b')
      expect(s.historicalRun?.error).toBe('oops')
    })

    it('returnToLive clears history selection', () => {
      workflowExecutionStore.store.setState({
        viewMode: 'history',
        selectedHistoricalRunId: 'run-a',
        historicalRun: {
          id: 'run-a',
          workflow_id: 'w1',
          status: 'completed',
          started_at: null,
          completed_at: null,
          outputs: null,
          error: null,
        },
      })

      workflowExecutionStore.returnToLive()

      const s = getState()
      expect(s.viewMode).toBe('live')
      expect(s.selectedHistoricalRunId).toBeNull()
      expect(s.historicalRun).toBeNull()
    })

    it('STARTED event auto-switches to live mode', () => {
      workflowExecutionStore.store.setState({
        viewMode: 'history',
        selectedHistoricalRunId: 'run-old',
        historicalRun: {
          id: 'run-old',
          workflow_id: 'w1',
          status: 'completed',
          started_at: null,
          completed_at: null,
          outputs: null,
          error: null,
        },
      })

      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 2 }))

      const s = getState()
      expect(s.viewMode).toBe('live')
      expect(s.selectedHistoricalRunId).toBeNull()
      expect(s.historicalRun).toBeNull()
    })

    it('reset clears history state', () => {
      workflowExecutionStore.store.setState({
        viewMode: 'history',
        runs: [{ id: 'run-a', workflow_id: 'w1', status: 'completed', started_at: null, completed_at: null, outputs: null, error: null }],
        selectedHistoricalRunId: 'run-a',
        historicalRun: {
          id: 'run-a',
          workflow_id: 'w1',
          status: 'completed',
          started_at: null,
          completed_at: null,
          outputs: null,
          error: null,
        },
        historyLoading: true,
        historyError: 'some error',
      })

      workflowExecutionStore.reset()

      const s = getState()
      expect(s.viewMode).toBe('live')
      expect(s.runs).toEqual([])
      expect(s.selectedHistoricalRunId).toBeNull()
      expect(s.historicalRun).toBeNull()
      expect(s.historyLoading).toBe(false)
      expect(s.historyError).toBeNull()
    })
  })

  describe('sub-workflow events', () => {
    it('SUB_WORKFLOW_STARTED initializes subWorkflowProgress on parent step', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 2 }))
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 'parent-1', step_name: 'Sub Step', agent_id: null, execution_id: null }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_STARTED, {
          workflow_id: 'w1',
          parent_step_id: 'parent-1',
          child_execution_id: 'child-exec-1',
          total_steps: 3,
        }),
      )

      const step = getState().stepStates['parent-1']
      expect(step.subWorkflowProgress).toBeDefined()
      expect(step.subWorkflowProgress!.childExecutionId).toBe('child-exec-1')
      expect(step.subWorkflowProgress!.totalSteps).toBe(3)
      expect(step.subWorkflowProgress!.completedSteps).toBe(0)
      expect(step.subWorkflowProgress!.status).toBe('running')
      expect(step.subWorkflowProgress!.childSteps).toEqual([])
    })

    it('SUB_WORKFLOW_STEP_PROGRESS adds child step on started status', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 'p1', step_name: 'Sub', agent_id: null, execution_id: null }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_STARTED, { workflow_id: 'w1', parent_step_id: 'p1', child_execution_id: 'ce1', total_steps: 2 }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_STEP_PROGRESS, {
          workflow_id: 'w1',
          parent_step_id: 'p1',
          child_execution_id: 'ce1',
          child_step_id: 'cs1',
          child_step_name: 'Designer',
          status: 'started',
        }),
      )

      const progress = getState().stepStates['p1'].subWorkflowProgress!
      expect(progress.childSteps).toHaveLength(1)
      expect(progress.childSteps[0].childStepId).toBe('cs1')
      expect(progress.childSteps[0].childStepName).toBe('Designer')
      expect(progress.childSteps[0].status).toBe('running')
      expect(progress.completedSteps).toBe(0)
    })

    it('SUB_WORKFLOW_STEP_PROGRESS updates existing child on completed', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 'p1', step_name: 'Sub', agent_id: null, execution_id: null }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_STARTED, { workflow_id: 'w1', parent_step_id: 'p1', child_execution_id: 'ce1', total_steps: 2 }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_STEP_PROGRESS, {
          workflow_id: 'w1', parent_step_id: 'p1', child_execution_id: 'ce1',
          child_step_id: 'cs1', child_step_name: 'Designer', status: 'started',
        }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_STEP_PROGRESS, {
          workflow_id: 'w1', parent_step_id: 'p1', child_execution_id: 'ce1',
          child_step_id: 'cs1', child_step_name: 'Designer', status: 'completed',
          input_tokens: 200, output_tokens: 100, duration_ms: 3000,
        }),
      )

      const progress = getState().stepStates['p1'].subWorkflowProgress!
      expect(progress.childSteps).toHaveLength(1)
      expect(progress.childSteps[0].status).toBe('success')
      expect(progress.childSteps[0].inputTokens).toBe(200)
      expect(progress.childSteps[0].outputTokens).toBe(100)
      expect(progress.childSteps[0].durationMs).toBe(3000)
      expect(progress.completedSteps).toBe(1)
    })

    it('SUB_WORKFLOW_STEP_PROGRESS handles failed child step', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 'p1', step_name: 'Sub', agent_id: null, execution_id: null }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_STARTED, { workflow_id: 'w1', parent_step_id: 'p1', child_execution_id: 'ce1', total_steps: 2 }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_STEP_PROGRESS, {
          workflow_id: 'w1', parent_step_id: 'p1', child_execution_id: 'ce1',
          child_step_id: 'cs1', child_step_name: 'Agent 1', status: 'started',
        }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_STEP_PROGRESS, {
          workflow_id: 'w1', parent_step_id: 'p1', child_execution_id: 'ce1',
          child_step_id: 'cs1', child_step_name: 'Agent 1', status: 'failed',
          error: 'LLM timeout',
        }),
      )

      const progress = getState().stepStates['p1'].subWorkflowProgress!
      expect(progress.childSteps[0].status).toBe('error')
      expect(progress.childSteps[0].error).toBe('LLM timeout')
      expect(progress.completedSteps).toBe(1)
    })

    it('SUB_WORKFLOW_COMPLETED marks sub-workflow as completed', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 'p1', step_name: 'Sub', agent_id: null, execution_id: null }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_STARTED, { workflow_id: 'w1', parent_step_id: 'p1', child_execution_id: 'ce1', total_steps: 1 }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_COMPLETED, {
          workflow_id: 'w1', parent_step_id: 'p1', child_execution_id: 'ce1', status: 'completed',
        }),
      )

      expect(getState().stepStates['p1'].subWorkflowProgress!.status).toBe('completed')
    })

    it('SUB_WORKFLOW_COMPLETED marks sub-workflow as failed', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 'p1', step_name: 'Sub', agent_id: null, execution_id: null }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_STARTED, { workflow_id: 'w1', parent_step_id: 'p1', child_execution_id: 'ce1', total_steps: 1 }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_COMPLETED, {
          workflow_id: 'w1', parent_step_id: 'p1', child_execution_id: 'ce1', status: 'failed',
        }),
      )

      expect(getState().stepStates['p1'].subWorkflowProgress!.status).toBe('failed')
    })

    it('SUB_WORKFLOW_STEP_PROGRESS no-ops if subWorkflowProgress is null', () => {
      workflowExecutionStore.handleWsEvent(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 'p1', step_name: 'Sub', agent_id: null, execution_id: null }),
      )
      // No SUB_WORKFLOW_STARTED — subWorkflowProgress is null
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.SUB_WORKFLOW_STEP_PROGRESS, {
          workflow_id: 'w1', parent_step_id: 'p1', child_execution_id: 'ce1',
          child_step_id: 'cs1', child_step_name: 'Agent', status: 'started',
        }),
      )

      expect(getState().stepStates['p1'].subWorkflowProgress).toBeNull()
    })
  })

  describe('selectors', () => {
    it('selectStepState returns step or undefined', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'Step One',
          agent_id: null,
          execution_id: null,
        }),
      )

      expect(workflowExecutionStore.selectStepState('s1')(getState())?.status).toBe('running')
      expect(workflowExecutionStore.selectStepState('missing')(getState())).toBeUndefined()
    })

    it('selectCompletedStepCount counts success steps', () => {
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'A',
          agent_id: null,
          output: null,
          input_tokens: null,
          output_tokens: null,
          duration_ms: null,
        }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, {
          workflow_id: 'w1',
          step_id: 's2',
          step_name: 'B',
          agent_id: null,
          output: null,
          input_tokens: null,
          output_tokens: null,
          duration_ms: null,
        }),
      )
      workflowExecutionStore.handleWsEvent(
        makeMsg(WORKFLOW_EVENT.STEP_FAILED, {
          workflow_id: 'w1',
          step_id: 's3',
          step_name: 'C',
          error: 'fail',
        }),
      )

      expect(workflowExecutionStore.selectCompletedStepCount(getState())).toBe(2)
    })
  })
})
