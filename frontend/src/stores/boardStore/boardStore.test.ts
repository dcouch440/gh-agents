import { boardStore } from './index'
import { workflowStore } from '../workflowStore'
import { nmGet, nmSet, createNormalizedMap } from '../lib'
import { mergeElementStepMap, mergeElementEdgeMap } from './submit'
import { INITIAL_STATE } from './_store'
import type { BoardSubmitResponse, PhaseZeroResponse } from '@/types/board'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'

// ── API Mock ──────────────────────────────────────────────────────────────

const { mockSubmitBoard } = vi.hoisted(() => ({
  mockSubmitBoard: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    workflows: { submitBoard: mockSubmitBoard },
  },
}))

// ── Fixtures ──────────────────────────────────────────────────────────────

const makeStep = (id: string, elementId: string): WorkflowStep & { element_id: string } => ({
  id,
  element_id: elementId,
  workflow_id: 'wf-1',
  agent_id: '',
  execution_mode: 'workforce',
  for_each_ref: null,
  prompt_template_id: null,
  prompt_template: 'test prompt',
  output_schema_id: null,
  output_variable_name: null,
  interactive_agent_id: null,
  for_each_label_field: null,
  display_order: 0,
  version: 1,
  reasoning_trace: false,
  verification_agent_ids: [],
  position_x: null,
  position_y: null,
  width: null,
  height: null,
  name: 'Test Step',
  room_id: null,
  system_prompt_suffix: null,
  description: '',

  pinned: false,
  run_results_summary: '',
  designer_handoff: '',
})

const emptyPhaseZero: PhaseZeroResponse = {
  created_steps: [],
  created_edges: [],
  deleted_steps: [],
  deleted_edges: [],
  rewired_edges: [],
  moved_steps: [],
  updated_steps: [],
}

const makeResponse = (overrides: Partial<BoardSubmitResponse> = {}): BoardSubmitResponse => ({
  is_first_submit: true,
  changeset: {
    agentless: { deleted_node_ids: [], deleted_edge_ids: [], rewired_edges: [], moved_nodes: [] },
    noise: [],
    meaningful: [],
    aggregate_score: 0,
    should_dispatch: false,
  },
  snapshot: { nodes: [], edges: [], global_notes: [] },
  phase_zero: emptyPhaseZero,
  dispatches: [],
  ...overrides,
})

// ── Setup / Teardown ──────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks()
  boardStore.store.setState(INITIAL_STATE)
})

// ── Tests ─────────────────────────────────────────────────────────────────

describe('boardStore', () => {
  describe('initial state', () => {
    it('starts idle with no data', () => {
      const s = boardStore.store.getState()
      expect(s.status).toBe('idle')
      expect(s.error).toBeNull()
      expect(s.lastResponse).toBeNull()
      expect(s.isFirstSubmit).toBe(false)
      expect(s.elementStepMap).toEqual({})
      expect(s.elementEdgeMap).toEqual({})
    })
  })

  describe('selectors', () => {
    it('selectStatus returns current status', () => {
      expect(boardStore.selectStatus(boardStore.store.getState())).toBe('idle')
    })

    it('selectIsSubmitting returns true during submit', () => {
      boardStore.store.setState({ status: 'submitting' })
      expect(boardStore.selectIsSubmitting(boardStore.store.getState())).toBe(true)
    })

    it('selectIsSubmitting returns false when idle', () => {
      expect(boardStore.selectIsSubmitting(boardStore.store.getState())).toBe(false)
    })

    it('selectError returns null initially', () => {
      expect(boardStore.selectError(boardStore.store.getState())).toBeNull()
    })
  })

  describe('submitBoard', () => {
    it('sets status to submitting during request', async () => {
      let resolvePromise: (v: BoardSubmitResponse) => void
      mockSubmitBoard.mockImplementation(
        () => new Promise<BoardSubmitResponse>((r) => { resolvePromise = r }),
      )

      const promise = boardStore.submitBoard('wf-1', [])
      expect(boardStore.store.getState().status).toBe('submitting')
      expect(boardStore.store.getState().error).toBeNull()

      resolvePromise!(makeResponse())
      await promise

      expect(boardStore.store.getState().status).toBe('success')
    })

    it('stores last response on success', async () => {
      const response = makeResponse({ is_first_submit: true })
      mockSubmitBoard.mockResolvedValue(response)

      await boardStore.submitBoard('wf-1', [])

      const s = boardStore.store.getState()
      expect(s.lastResponse).toBe(response)
      expect(s.isFirstSubmit).toBe(true)
      expect(s.error).toBeNull()
    })

    it('sets error status on failure', async () => {
      mockSubmitBoard.mockRejectedValue(new Error('Network timeout'))

      await boardStore.submitBoard('wf-1', [])

      const s = boardStore.store.getState()
      expect(s.status).toBe('error')
      expect(s.error).toBe('Network timeout')
      expect(s.lastResponse).toBeNull()
    })

    it('passes workflowId and elements to API', async () => {
      mockSubmitBoard.mockResolvedValue(makeResponse())

      const elements = [{ id: 'el-1', type: 'rectangle' }]
      await boardStore.submitBoard('wf-123', elements)

      expect(mockSubmitBoard).toHaveBeenCalledWith('wf-123', elements)
    })
  })

  describe('selective sync into workflowStore', () => {
    it('upserts created steps into workflowStore', async () => {
      const step = makeStep('step-1', 'el-1')
      const response = makeResponse({
        phase_zero: { ...emptyPhaseZero, created_steps: [step] },
      })
      mockSubmitBoard.mockResolvedValue(response)

      // Seed workflowStore with empty steps
      workflowStore.store.setState({ steps: createNormalizedMap() })

      await boardStore.submitBoard('wf-1', [])

      const synced = nmGet(workflowStore.store.getState().steps, 'step-1')
      expect(synced).toBeDefined()
      expect(synced!.name).toBe('Test Step')
    })

    it('preserves existing steps not in response', async () => {
      const existingStep: WorkflowStep = {
        ...makeStep('existing-1', 'el-0'),
        name: 'Existing',
      }
      const steps = nmSet(createNormalizedMap<WorkflowStep>(), 'existing-1', existingStep)
      workflowStore.store.setState({ steps })

      const newStep = makeStep('step-2', 'el-2')
      const response = makeResponse({
        phase_zero: { ...emptyPhaseZero, created_steps: [newStep] },
      })
      mockSubmitBoard.mockResolvedValue(response)

      await boardStore.submitBoard('wf-1', [])

      const state = workflowStore.store.getState()
      expect(nmGet(state.steps, 'existing-1')).toBeDefined()
      expect(nmGet(state.steps, 'existing-1')!.name).toBe('Existing')
      expect(nmGet(state.steps, 'step-2')).toBeDefined()
    })

    it('deletes removed steps from workflowStore', async () => {
      const existingStep: WorkflowStep = makeStep('doomed-1', 'el-d')
      const steps = nmSet(createNormalizedMap<WorkflowStep>(), 'doomed-1', existingStep)
      workflowStore.store.setState({ steps })

      const response = makeResponse({
        phase_zero: { ...emptyPhaseZero, deleted_steps: ['doomed-1'] },
      })
      mockSubmitBoard.mockResolvedValue(response)

      await boardStore.submitBoard('wf-1', [])

      expect(nmGet(workflowStore.store.getState().steps, 'doomed-1')).toBeUndefined()
    })

    it('deletes removed edges from workflowStore', async () => {
      const edge: WorkflowStepEdge = {
        id: 'edge-1',
        workflow_id: 'wf-1',
        from_step_id: 's1',
        to_step_id: 's2',
        from_output_port: null,
        to_input_port: null,
        edge_label: null,
      }
      const edges = nmSet(createNormalizedMap<WorkflowStepEdge>(), 'edge-1', edge)
      workflowStore.store.setState({ edges })

      const response = makeResponse({
        phase_zero: { ...emptyPhaseZero, deleted_edges: ['edge-1'] },
      })
      mockSubmitBoard.mockResolvedValue(response)

      await boardStore.submitBoard('wf-1', [])

      expect(nmGet(workflowStore.store.getState().edges, 'edge-1')).toBeUndefined()
    })

    it('upserts updated steps into workflowStore', async () => {
      const existingStep: WorkflowStep = { ...makeStep('step-u', 'el-u'), name: 'Old Name' }
      const steps = nmSet(createNormalizedMap<WorkflowStep>(), 'step-u', existingStep)
      workflowStore.store.setState({ steps })

      const updatedStep = { ...makeStep('step-u', 'el-u'), name: 'New Name' }
      const response = makeResponse({
        phase_zero: { ...emptyPhaseZero, updated_steps: [updatedStep] },
      })
      mockSubmitBoard.mockResolvedValue(response)

      await boardStore.submitBoard('wf-1', [])

      const synced = nmGet(workflowStore.store.getState().steps, 'step-u')
      expect(synced!.name).toBe('New Name')
    })
  })

  describe('element map accumulation', () => {
    it('accumulates step mappings across submits', async () => {
      const step1 = makeStep('step-1', 'el-1')
      const response1 = makeResponse({
        phase_zero: { ...emptyPhaseZero, created_steps: [step1] },
      })
      mockSubmitBoard.mockResolvedValue(response1)
      workflowStore.store.setState({ steps: createNormalizedMap() })

      await boardStore.submitBoard('wf-1', [])
      expect(boardStore.store.getState().elementStepMap).toEqual({ 'el-1': 'step-1' })

      const step2 = makeStep('step-2', 'el-2')
      const response2 = makeResponse({
        phase_zero: { ...emptyPhaseZero, created_steps: [step2] },
      })
      mockSubmitBoard.mockResolvedValue(response2)

      await boardStore.submitBoard('wf-1', [])
      expect(boardStore.store.getState().elementStepMap).toEqual({
        'el-1': 'step-1',
        'el-2': 'step-2',
      })
    })

    it('accumulates edge mappings from created_edges', async () => {
      const response = makeResponse({
        phase_zero: {
          ...emptyPhaseZero,
          created_edges: [{ element_id: 'el-e1', edge_id: 'edge-1', from_step_id: 's1', to_step_id: 's2' }],
        },
      })
      mockSubmitBoard.mockResolvedValue(response)
      workflowStore.store.setState({ steps: createNormalizedMap() })

      await boardStore.submitBoard('wf-1', [])
      expect(boardStore.store.getState().elementEdgeMap).toEqual({ 'el-e1': 'edge-1' })
    })

    it('removes deleted step mappings', async () => {
      boardStore.store.setState({ elementStepMap: { 'el-1': 'step-1', 'el-2': 'step-2' } })

      const response = makeResponse({
        phase_zero: { ...emptyPhaseZero, deleted_steps: ['el-1'] },
      })
      mockSubmitBoard.mockResolvedValue(response)
      workflowStore.store.setState({ steps: createNormalizedMap() })

      await boardStore.submitBoard('wf-1', [])
      expect(boardStore.store.getState().elementStepMap).toEqual({ 'el-2': 'step-2' })
    })
  })

  describe('resetBoard', () => {
    it('returns to initial state', async () => {
      mockSubmitBoard.mockResolvedValue(makeResponse())
      workflowStore.store.setState({ steps: createNormalizedMap() })
      await boardStore.submitBoard('wf-1', [])

      boardStore.resetBoard()

      const s = boardStore.store.getState()
      expect(s.status).toBe('idle')
      expect(s.lastResponse).toBeNull()
      expect(s.elementStepMap).toEqual({})
      expect(s.elementEdgeMap).toEqual({})
    })
  })

  describe('mergeElementStepMap (unit)', () => {
    it('returns existing map when no new entries', () => {
      const existing = { 'el-1': 'step-1' }
      const result = mergeElementStepMap(existing, emptyPhaseZero)
      expect(result).toBe(existing)
    })

    it('merges new entries', () => {
      const existing = { 'el-1': 'step-1' }
      const phaseZero: PhaseZeroResponse = {
        ...emptyPhaseZero,
        created_steps: [makeStep('step-2', 'el-2')],
      }
      const result = mergeElementStepMap(existing, phaseZero)
      expect(result).toEqual({ 'el-1': 'step-1', 'el-2': 'step-2' })
    })

    it('overwrites existing entries from updated steps', () => {
      const existing = { 'el-1': 'old-step' }
      const phaseZero: PhaseZeroResponse = {
        ...emptyPhaseZero,
        updated_steps: [makeStep('new-step', 'el-1')],
      }
      const result = mergeElementStepMap(existing, phaseZero)
      expect(result).toEqual({ 'el-1': 'new-step' })
    })
  })

  describe('mergeElementEdgeMap (unit)', () => {
    it('returns existing map when no changes', () => {
      const existing = { 'el-e1': 'edge-1' }
      const result = mergeElementEdgeMap(existing, emptyPhaseZero)
      expect(result).toBe(existing)
    })

    it('merges new edge entries', () => {
      const existing = {}
      const phaseZero: PhaseZeroResponse = {
        ...emptyPhaseZero,
        created_edges: [{ element_id: 'el-e1', edge_id: 'edge-1', from_step_id: 's1', to_step_id: 's2' }],
      }
      const result = mergeElementEdgeMap(existing, phaseZero)
      expect(result).toEqual({ 'el-e1': 'edge-1' })
    })

    it('removes deleted edge entries', () => {
      const existing = { 'el-e1': 'edge-1', 'el-e2': 'edge-2' }
      const phaseZero: PhaseZeroResponse = {
        ...emptyPhaseZero,
        deleted_edges: ['el-e1'],
      }
      const result = mergeElementEdgeMap(existing, phaseZero)
      expect(result).toEqual({ 'el-e2': 'edge-2' })
    })
  })
})
