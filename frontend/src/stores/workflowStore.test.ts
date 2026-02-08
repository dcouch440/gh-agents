import { workflowStore } from './workflowStore'
import { nmSize, nmGet, createNormalizedMap } from './lib'
import type { Workflow, WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import type { Document } from '@/types/document'

const {
  mockList,
  mockGet,
  mockCreate,
  mockUpdate,
  mockDelete,
  mockListSteps,
  mockCreateStep,
  mockUpdateStep,
  mockDeleteStep,
  mockListEdges,
  mockCreateEdge,
  mockDeleteEdge,
  mockListStepDocuments,
  mockAddStepDocument,
  mockRemoveStepDocument,
} = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockGet: vi.fn(),
  mockCreate: vi.fn(),
  mockUpdate: vi.fn(),
  mockDelete: vi.fn(),
  mockListSteps: vi.fn(),
  mockCreateStep: vi.fn(),
  mockUpdateStep: vi.fn(),
  mockDeleteStep: vi.fn(),
  mockListEdges: vi.fn(),
  mockCreateEdge: vi.fn(),
  mockDeleteEdge: vi.fn(),
  mockListStepDocuments: vi.fn(),
  mockAddStepDocument: vi.fn(),
  mockRemoveStepDocument: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    workflows: {
      list: mockList,
      get: mockGet,
      create: mockCreate,
      update: mockUpdate,
      delete: mockDelete,
      listSteps: mockListSteps,
      createStep: mockCreateStep,
      getStep: vi.fn(),
      updateStep: mockUpdateStep,
      deleteStep: mockDeleteStep,
      listEdges: mockListEdges,
      createEdge: mockCreateEdge,
      deleteEdge: mockDeleteEdge,
      listStepDocuments: mockListStepDocuments,
      addStepDocument: mockAddStepDocument,
      removeStepDocument: mockRemoveStepDocument,
    },
  },
}))

const wf1: Workflow = {
  id: 'wf1',
  name: 'Test Workflow',
  description: null,
  created_at: '2025-01-01T00:00:00Z',
  container_enabled: false,
  target_repo_url: null,
  target_branch: null,
  vpn_enabled: false,
}

const wf2: Workflow = {
  ...wf1,
  id: 'wf2',
  name: 'Second Workflow',
}

const step1: WorkflowStep = {
  id: 's1',
  workflow_id: 'wf1',
  agent_id: 'a1',
  execution_mode: 'single',
  for_each_ref: null,
  prompt_template_id: null,
  prompt_template: '',
  output_schema_id: null,
  output_variable_name: null,
  interactive_agent_id: null,
  for_each_label_field: null,
  display_order: 0,
  version: 1,
  reasoning_trace: false,
  verification_agent_ids: [],
  position_x: 0,
  position_y: 0,
  name: 'Step 1',
}

const step2: WorkflowStep = {
  ...step1,
  id: 's2',
  name: 'Step 2',
  agent_id: 'a2',
  position_x: 200,
}

const edge1: WorkflowStepEdge = {
  id: 'e1',
  from_step_id: 's1',
  to_step_id: 's2',
}

const doc1: Document = {
  id: 'd1',
  user_id: 'u1',
  title: 'Doc 1',
  content: 'content',
  content_type: 'text/plain',
  source_url: null,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

const resetStore = () => {
  workflowStore.store.setState({
    items: createNormalizedMap(),
    activeWorkflowId: null,
    steps: [],
    edges: [],
    documentsByStep: {},
    loading: false,
    error: null,
    dirty: false,
    lastFetched: null,
  })
}

beforeEach(() => {
  vi.clearAllMocks()
  resetStore()
})

describe('workflowStore', () => {
  describe('workflow CRUD', () => {
    it('fetchAll populates items', async () => {
      mockList.mockResolvedValue([wf1, wf2])
      await workflowStore.fetchAll()

      const s = workflowStore.store.getState()
      expect(nmSize(s.items)).toBe(2)
      expect(nmGet(s.items, 'wf1')).toEqual(wf1)
      expect(s.loading).toBe(false)
    })

    it('fetchAll sets error on failure', async () => {
      mockList.mockRejectedValue(new Error('Network error'))
      await workflowStore.fetchAll()

      expect(workflowStore.store.getState().error).toBe('Network error')
    })

    it('fetchOne upserts', async () => {
      mockGet.mockResolvedValue(wf1)
      const result = await workflowStore.fetchOne('wf1')

      expect(result).toEqual(wf1)
      expect(nmGet(workflowStore.store.getState().items, 'wf1')).toEqual(wf1)
    })

    it('create upserts and returns', async () => {
      mockCreate.mockResolvedValue(wf1)
      const result = await workflowStore.create({ name: 'Test Workflow' })

      expect(result).toEqual(wf1)
      expect(nmGet(workflowStore.store.getState().items, 'wf1')).toEqual(wf1)
    })

    it('update upserts', async () => {
      const updated = { ...wf1, name: 'Updated' }
      mockUpdate.mockResolvedValue(updated)
      const result = await workflowStore.update('wf1', { name: 'Updated' })

      expect(result.name).toBe('Updated')
    })

    it('remove optimistically deletes and rolls back on failure', async () => {
      mockList.mockResolvedValue([wf1])
      await workflowStore.fetchAll()
      expect(nmSize(workflowStore.store.getState().items)).toBe(1)

      mockDelete.mockRejectedValue(new Error('Delete failed'))
      await expect(workflowStore.remove('wf1')).rejects.toThrow('Delete failed')

      expect(nmSize(workflowStore.store.getState().items)).toBe(1)
    })
  })

  describe('loadWorkflow', () => {
    it('fetches workflow, steps, and edges in parallel', async () => {
      mockGet.mockResolvedValue(wf1)
      mockListSteps.mockResolvedValue([step1, step2])
      mockListEdges.mockResolvedValue([edge1])

      await workflowStore.loadWorkflow('wf1')

      const s = workflowStore.store.getState()
      expect(s.activeWorkflowId).toBe('wf1')
      expect(s.steps).toEqual([step1, step2])
      expect(s.edges).toEqual([edge1])
      expect(s.documentsByStep).toEqual({})
      expect(s.loading).toBe(false)
      expect(s.dirty).toBe(false)
    })

    it('sets error on failure', async () => {
      mockGet.mockRejectedValue(new Error('Not found'))
      mockListSteps.mockRejectedValue(new Error('Not found'))
      mockListEdges.mockRejectedValue(new Error('Not found'))

      await workflowStore.loadWorkflow('wf1')

      expect(workflowStore.store.getState().error).toBe('Not found')
    })
  })

  describe('clearActive', () => {
    it('resets active context', async () => {
      mockGet.mockResolvedValue(wf1)
      mockListSteps.mockResolvedValue([step1])
      mockListEdges.mockResolvedValue([edge1])
      await workflowStore.loadWorkflow('wf1')

      workflowStore.clearActive()

      const s = workflowStore.store.getState()
      expect(s.activeWorkflowId).toBeNull()
      expect(s.steps).toEqual([])
      expect(s.edges).toEqual([])
    })
  })

  describe('steps', () => {
    beforeEach(async () => {
      mockGet.mockResolvedValue(wf1)
      mockListSteps.mockResolvedValue([step1])
      mockListEdges.mockResolvedValue([])
      await workflowStore.loadWorkflow('wf1')
    })

    it('createStep appends and sets dirty', async () => {
      mockCreateStep.mockResolvedValue(step2)
      const result = await workflowStore.createStep({ name: 'Step 2', execution_mode: 'single' })

      expect(result).toEqual(step2)
      expect(workflowStore.store.getState().steps).toHaveLength(2)
      expect(workflowStore.store.getState().dirty).toBe(true)
    })

    it('updateStep replaces in list', async () => {
      const updated = { ...step1, name: 'Updated Step' }
      mockUpdateStep.mockResolvedValue(updated)
      await workflowStore.updateStep('s1', { name: 'Updated Step' })

      expect(workflowStore.store.getState().steps[0].name).toBe('Updated Step')
    })

    it('deleteStep filters from steps and removes connected edges', async () => {
      // Add step2 and an edge first
      workflowStore.store.setState((s) => ({
        steps: [...s.steps, step2],
        edges: [edge1],
      }))
      mockDeleteStep.mockResolvedValue(undefined)

      await workflowStore.deleteStep('s1')

      const s = workflowStore.store.getState()
      expect(s.steps).toHaveLength(1)
      expect(s.steps[0].id).toBe('s2')
      expect(s.edges).toHaveLength(0) // edge connected to s1 removed
    })

    it('createStep returns null when no active workflow', async () => {
      workflowStore.clearActive()
      const result = await workflowStore.createStep({ name: 'X', execution_mode: 'single' })
      expect(result).toBeNull()
    })
  })

  describe('edges', () => {
    beforeEach(async () => {
      mockGet.mockResolvedValue(wf1)
      mockListSteps.mockResolvedValue([step1, step2])
      mockListEdges.mockResolvedValue([])
      await workflowStore.loadWorkflow('wf1')
    })

    it('addEdge appends', async () => {
      mockCreateEdge.mockResolvedValue(edge1)
      const result = await workflowStore.addEdge({ from_step_id: 's1', to_step_id: 's2' })

      expect(result).toEqual(edge1)
      expect(workflowStore.store.getState().edges).toHaveLength(1)
      expect(workflowStore.store.getState().dirty).toBe(true)
    })

    it('removeEdge filters', async () => {
      workflowStore.store.setState({ edges: [edge1] })
      mockDeleteEdge.mockResolvedValue(undefined)

      await workflowStore.removeEdge('e1')

      expect(workflowStore.store.getState().edges).toHaveLength(0)
    })
  })

  describe('step documents', () => {
    beforeEach(async () => {
      mockGet.mockResolvedValue(wf1)
      mockListSteps.mockResolvedValue([step1])
      mockListEdges.mockResolvedValue([])
      await workflowStore.loadWorkflow('wf1')
    })

    it('fetchStepDocuments stores by stepId', async () => {
      mockListStepDocuments.mockResolvedValue([doc1])
      await workflowStore.fetchStepDocuments('s1')

      const docs = workflowStore.selectStepDocuments('s1')(workflowStore.store.getState())
      expect(docs).toEqual([doc1])
    })

    it('addStepDocument calls API and refetches', async () => {
      mockAddStepDocument.mockResolvedValue(undefined)
      mockListStepDocuments.mockResolvedValue([doc1])

      await workflowStore.addStepDocument('s1', 'd1')

      expect(mockAddStepDocument).toHaveBeenCalledWith('wf1', 's1', 'd1')
      expect(mockListStepDocuments).toHaveBeenCalledWith('wf1', 's1')
    })

    it('removeStepDocument calls API and refetches', async () => {
      mockRemoveStepDocument.mockResolvedValue(undefined)
      mockListStepDocuments.mockResolvedValue([])

      await workflowStore.removeStepDocument('s1', 'd1')

      expect(mockRemoveStepDocument).toHaveBeenCalledWith('wf1', 's1', 'd1')
    })
  })

  describe('selectors', () => {
    it('selectAll returns array', async () => {
      mockList.mockResolvedValue([wf1, wf2])
      await workflowStore.fetchAll()

      expect(workflowStore.selectAll(workflowStore.store.getState())).toHaveLength(2)
    })

    it('selectById returns undefined for missing', () => {
      expect(workflowStore.selectById('missing')(workflowStore.store.getState())).toBeUndefined()
    })

    it('selectStepDocuments returns stable empty array for missing', () => {
      const a = workflowStore.selectStepDocuments('missing')(workflowStore.store.getState())
      const b = workflowStore.selectStepDocuments('missing')(workflowStore.store.getState())
      expect(a).toEqual([])
      expect(a).toBe(b) // Same reference
    })

    it('selectDirty returns false initially', () => {
      expect(workflowStore.selectDirty(workflowStore.store.getState())).toBe(false)
    })
  })

  describe('stale data', () => {
    it('starts as stale (lastFetched is null)', () => {
      expect(workflowStore.selectIsStale(workflowStore.store.getState())).toBe(true)
      expect(workflowStore.store.getState().lastFetched).toBeNull()
    })

    it('is not stale after fetchAll', async () => {
      mockList.mockResolvedValue([wf1])
      await workflowStore.fetchAll()

      expect(workflowStore.selectIsStale(workflowStore.store.getState())).toBe(false)
      expect(workflowStore.store.getState().lastFetched).toBeTypeOf('number')
    })

    it('fetchIfStale skips when fresh', async () => {
      mockList.mockResolvedValue([wf1])
      await workflowStore.fetchAll()
      expect(mockList).toHaveBeenCalledTimes(1)

      await workflowStore.fetchIfStale()
      expect(mockList).toHaveBeenCalledTimes(1)
    })
  })

  describe('sync mutations', () => {
    it('upsert adds workflow', () => {
      workflowStore.upsert(wf1)
      expect(nmGet(workflowStore.store.getState().items, 'wf1')).toEqual(wf1)
    })

    it('setDirty updates dirty flag', () => {
      workflowStore.setDirty(true)
      expect(workflowStore.store.getState().dirty).toBe(true)
    })
  })
})
