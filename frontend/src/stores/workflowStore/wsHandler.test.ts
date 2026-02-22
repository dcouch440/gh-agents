import type { WsWireMessage } from '@/types/ws'
import { handleWsEvent } from './wsHandler'
import { store } from './_store'

// ── Mocks ────────────────────────────────────────────────────────────────────

const mockGetStep = vi.hoisted(() => vi.fn())
vi.mock('@/api', () => ({
  api: {
    workflows: {
      getStep: mockGetStep,
    },
  },
}))

const makeMsg = (event: string, data: Record<string, unknown>): WsWireMessage => ({
  topic: 'workflow',
  event,
  ts: '2025-01-01T00:00:00Z',
  run_id: null,
  user_id: null,
  data,
})

// ── Tests ────────────────────────────────────────────────────────────────────

describe('workflowStore/wsHandler', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    store.setState({ activeWorkflowId: 'wf-1', dirtyStepIds: new Set<string>() })
  })

  describe('step_config_updated', () => {
    it('calls getStep for active workflow', () => {
      const mockStep = { id: 'step-1', name: 'Documenter', prompt_template: 'Write docs' }
      mockGetStep.mockResolvedValueOnce(mockStep)

      handleWsEvent(makeMsg('step_config_updated', { workflow_id: 'wf-1', step_id: 'step-1' }))

      expect(mockGetStep).toHaveBeenCalledWith('wf-1', 'step-1')
    })

    it('ignores events for a different workflow', () => {
      handleWsEvent(makeMsg('step_config_updated', { workflow_id: 'wf-other', step_id: 'step-1' }))
      expect(mockGetStep).not.toHaveBeenCalled()
    })
  })

  describe('plan_updated', () => {
    it('updates planByStep for active workflow', () => {
      store.setState({ activeWorkflowId: 'wf-1', planByStep: {} })
      handleWsEvent(makeMsg('plan_updated', {
        workflow_id: 'wf-1',
        step_id: 'step-1',
        content: '## Direction\n- Build auth',
      }))
      expect(store.getState().planByStep).toEqual({ 'step-1': '## Direction\n- Build auth' })
    })

    it('merges with existing plan entries', () => {
      store.setState({ activeWorkflowId: 'wf-1', planByStep: { 'step-2': 'existing plan' } })
      handleWsEvent(makeMsg('plan_updated', {
        workflow_id: 'wf-1',
        step_id: 'step-1',
        content: 'new plan',
      }))
      const plans = store.getState().planByStep
      expect(plans['step-1']).toBe('new plan')
      expect(plans['step-2']).toBe('existing plan')
    })

    it('replaces existing plan for same step', () => {
      store.setState({ activeWorkflowId: 'wf-1', planByStep: { 'step-1': 'old' } })
      handleWsEvent(makeMsg('plan_updated', {
        workflow_id: 'wf-1',
        step_id: 'step-1',
        content: 'updated',
      }))
      expect(store.getState().planByStep['step-1']).toBe('updated')
    })

    it('ignores events for a different workflow', () => {
      store.setState({ activeWorkflowId: 'wf-1', planByStep: {} })
      handleWsEvent(makeMsg('plan_updated', {
        workflow_id: 'wf-other',
        step_id: 'step-1',
        content: 'should not appear',
      }))
      expect(store.getState().planByStep).toEqual({})
    })
  })

  describe('unknown events', () => {
    it('does not crash on unrecognized events', () => {
      expect(() => handleWsEvent(makeMsg('started', { workflow_id: 'wf-1', total_steps: 5 }))).not.toThrow()
      expect(mockGetStep).not.toHaveBeenCalled()
    })
  })

  describe('no active workflow', () => {
    it('ignores events when no workflow is active', () => {
      store.setState({ activeWorkflowId: null })
      handleWsEvent(makeMsg('step_config_updated', { workflow_id: 'wf-1', step_id: 'step-1' }))
      expect(mockGetStep).not.toHaveBeenCalled()
    })
  })
})
