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

  describe('assistant_notes_updated', () => {
    it('updates notesByStep for active workflow', () => {
      store.setState({ activeWorkflowId: 'wf-1', notesByStep: {} })
      handleWsEvent(makeMsg('assistant_notes_updated', {
        workflow_id: 'wf-1',
        step_id: 'step-1',
        content: '## Direction\n- Build auth',
      }))
      expect(store.getState().notesByStep).toEqual({ 'step-1': '## Direction\n- Build auth' })
    })

    it('merges with existing notes', () => {
      store.setState({ activeWorkflowId: 'wf-1', notesByStep: { 'step-2': 'existing notes' } })
      handleWsEvent(makeMsg('assistant_notes_updated', {
        workflow_id: 'wf-1',
        step_id: 'step-1',
        content: 'new notes',
      }))
      const notes = store.getState().notesByStep
      expect(notes['step-1']).toBe('new notes')
      expect(notes['step-2']).toBe('existing notes')
    })

    it('replaces existing notes for same step', () => {
      store.setState({ activeWorkflowId: 'wf-1', notesByStep: { 'step-1': 'old' } })
      handleWsEvent(makeMsg('assistant_notes_updated', {
        workflow_id: 'wf-1',
        step_id: 'step-1',
        content: 'updated',
      }))
      expect(store.getState().notesByStep['step-1']).toBe('updated')
    })

    it('ignores events for a different workflow', () => {
      store.setState({ activeWorkflowId: 'wf-1', notesByStep: {} })
      handleWsEvent(makeMsg('assistant_notes_updated', {
        workflow_id: 'wf-other',
        step_id: 'step-1',
        content: 'should not appear',
      }))
      expect(store.getState().notesByStep).toEqual({})
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
