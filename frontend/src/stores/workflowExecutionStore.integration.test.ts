import { workflowExecutionStore } from './workflowExecutionStore'
import { WORKFLOW_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'

const getState = () => workflowExecutionStore.store.getState()

let tsCounter = 0
const makeMsg = (event: string, data: Record<string, unknown>, runId = 'run-1'): WsWireMessage => ({
  topic: 'workflow',
  event,
  ts: `2025-01-01T00:00:${String(tsCounter++).padStart(2, '0')}Z`,
  run_id: runId,
  user_id: null,
  data,
})

const handle = workflowExecutionStore.handleWsEvent

beforeEach(() => {
  workflowExecutionStore.reset()
  tsCounter = 0
})

describe('workflowExecutionStore integration', () => {
  describe('full happy path', () => {
    it('STARTED → 3x(STEP_STARTED → STEP_COMPLETED) → COMPLETED', () => {
      handle(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 3 }))
      expect(getState().isRunning).toBe(true)
      expect(getState().totalSteps).toBe(3)

      for (const stepId of ['s1', 's2', 's3']) {
        handle(
          makeMsg(WORKFLOW_EVENT.STEP_STARTED, {
            workflow_id: 'w1',
            step_id: stepId,
            step_name: stepId,
            agent_id: null,
            execution_id: null,
          }),
        )
        expect(getState().stepStates[stepId].status).toBe('running')

        handle(
          makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, {
            workflow_id: 'w1',
            step_id: stepId,
            step_name: stepId,
            agent_id: null,
            output: `${stepId} done`,
            input_tokens: 10,
            output_tokens: 5,
            duration_ms: 100,
          }),
        )
        expect(getState().stepStates[stepId].status).toBe('success')
      }

      handle(makeMsg(WORKFLOW_EVENT.COMPLETED, { workflow_id: 'w1', duration_ms: 3000 }))

      const s = getState()
      expect(s.isRunning).toBe(false)
      expect(s.durationMs).toBe(3000)
      expect(workflowExecutionStore.selectCompletedStepCount(s)).toBe(3)
      expect(s.error).toBeNull()
    })
  })

  describe('for-each lifecycle', () => {
    it('tracks for-each progress across iterations', () => {
      handle(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))
      handle(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'ForEach',
          agent_id: null,
          execution_id: null,
        }),
      )

      for (let i = 1; i <= 3; i++) {
        handle(
          makeMsg(WORKFLOW_EVENT.FOR_EACH_PROGRESS, { workflow_id: 'w1', step_id: 's1', step_name: 'ForEach', completed: i, total: 3 }),
        )
        expect(getState().stepStates['s1'].forEachProgress).toEqual({ completed: i, total: 3 })
      }

      handle(
        makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'ForEach',
          agent_id: null,
          output: 'done',
          input_tokens: 30,
          output_tokens: 15,
          duration_ms: 500,
        }),
      )
      handle(makeMsg(WORKFLOW_EVENT.COMPLETED, { workflow_id: 'w1', duration_ms: 600 }))

      const s = getState()
      expect(s.isRunning).toBe(false)
      expect(s.stepStates['s1'].status).toBe('success')
      expect(s.stepStates['s1'].forEachProgress).toEqual({ completed: 3, total: 3 })
    })
  })

  describe('pause/resume lifecycle', () => {
    it('STARTED → STEP_STARTED → STEP_PAUSED → RESUMED → STEP_COMPLETED → COMPLETED', () => {
      handle(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))
      handle(
        makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 's1', step_name: 'Review', agent_id: null, execution_id: null }),
      )
      expect(getState().stepStates['s1'].status).toBe('running')

      handle(makeMsg(WORKFLOW_EVENT.STEP_PAUSED, { workflow_id: 'w1', step_id: 's1', step_name: 'Review' }))
      expect(getState().stepStates['s1'].status).toBe('paused')

      handle(makeMsg(WORKFLOW_EVENT.RESUMED, { workflow_id: 'w1', step_id: 's1' }))
      expect(getState().isRunning).toBe(true)
      expect(getState().stepStates['s1'].status).toBe('running')

      handle(
        makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'Review',
          agent_id: null,
          output: 'approved',
          input_tokens: 5,
          output_tokens: 2,
          duration_ms: 200,
        }),
      )
      handle(makeMsg(WORKFLOW_EVENT.COMPLETED, { workflow_id: 'w1', duration_ms: 300 }))

      expect(getState().isRunning).toBe(false)
      expect(getState().stepStates['s1'].status).toBe('success')
    })
  })

  describe('failure mid-run', () => {
    it('completes first step then fails on second', () => {
      handle(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 2 }))

      handle(makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 's1', step_name: 'A', agent_id: null, execution_id: null }))
      handle(
        makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, {
          workflow_id: 'w1',
          step_id: 's1',
          step_name: 'A',
          agent_id: null,
          output: 'ok',
          input_tokens: 10,
          output_tokens: 5,
          duration_ms: 100,
        }),
      )

      handle(makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 's2', step_name: 'B', agent_id: null, execution_id: null }))
      handle(makeMsg(WORKFLOW_EVENT.STEP_FAILED, { workflow_id: 'w1', step_id: 's2', step_name: 'B', error: 'LLM timeout' }))
      handle(makeMsg(WORKFLOW_EVENT.FAILED, { workflow_id: 'w1', error: 'LLM timeout' }))

      const s = getState()
      expect(s.isRunning).toBe(false)
      expect(s.error).toBe('LLM timeout')
      expect(s.stepStates['s1'].status).toBe('success')
      expect(s.stepStates['s2'].status).toBe('error')
      expect(s.stepStates['s2'].error).toBe('LLM timeout')
      expect(workflowExecutionStore.selectCompletedStepCount(s)).toBe(1)
    })
  })

  describe('malformed message resilience', () => {
    it('malformed STEP_STARTED (missing step_id) — does not crash', () => {
      handle(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))

      // Missing step_id — handler doesn't throw, but may create 'undefined' key
      handle(makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1' }))

      // The important thing: no crash, workflow still running
      expect(getState().isRunning).toBe(true)
    })

    it('unknown event type — state unchanged, no error', () => {
      handle(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 1 }))
      const before = getState()

      handle(makeMsg('totally_unknown', { foo: 'bar' }))

      expect(getState()).toBe(before)
    })

    it('interleaved valid + malformed — only valid events applied', () => {
      handle(makeMsg(WORKFLOW_EVENT.STARTED, { workflow_id: 'w1', total_steps: 2 }))

      handle(makeMsg(WORKFLOW_EVENT.STEP_STARTED, { workflow_id: 'w1', step_id: 's1', step_name: 'A', agent_id: null, execution_id: null }))

      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      // Malformed — no step_id
      handle(makeMsg(WORKFLOW_EVENT.STEP_COMPLETED, { workflow_id: 'w1' }))
      spy.mockRestore()

      // s1 should still be running (malformed STEP_COMPLETED didn't alter it)
      // The malformed message created an 'undefined' key, but s1 is untouched
      expect(getState().stepStates['s1'].status).toBe('running')

      // Now valid completion
      handle(
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
      expect(getState().stepStates['s1'].status).toBe('success')
    })
  })
})
