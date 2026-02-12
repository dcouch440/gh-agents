import { describe, it, expect, vi, beforeEach } from 'vitest'
import type { DocumentDef } from '@/types/workflow'

const {
  mockListDocumentDefs,
  mockCreateDocumentDef,
  mockDeleteDocumentDef,
} = vi.hoisted(() => ({
  mockListDocumentDefs: vi.fn(),
  mockCreateDocumentDef: vi.fn(),
  mockDeleteDocumentDef: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    workflows: {
      list: vi.fn(),
      get: vi.fn(),
      create: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
      listSteps: vi.fn(),
      createStep: vi.fn(),
      getStep: vi.fn(),
      updateStep: vi.fn(),
      deleteStep: vi.fn(),
      listEdges: vi.fn(),
      createEdge: vi.fn(),
      deleteEdge: vi.fn(),
      listStepDocuments: vi.fn(),
      addStepDocument: vi.fn(),
      removeStepDocument: vi.fn(),
      listDocumentDefs: mockListDocumentDefs,
      createDocumentDef: mockCreateDocumentDef,
      deleteDocumentDef: mockDeleteDocumentDef,
    },
  },
}))

// Must import after mocks
const { workflowStore } = await import('.')

const mockDefs: DocumentDef[] = [
  {
    id: 'def-001',
    step_id: 'step-1',
    name: 'README',
    description: 'Project readme',
    target_length: 5000,
    display_order: 0,
    created_at: '2025-01-01T00:00:00Z',
  },
  {
    id: 'def-002',
    step_id: 'step-1',
    name: 'CHANGELOG',
    description: null,
    target_length: 2000,
    display_order: 1,
    created_at: '2025-01-01T00:00:00Z',
  },
]

describe('workflowStore documents', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Set active workflow so actions can proceed
    workflowStore.store.setState({ activeWorkflowId: 'wf-001' })
  })

  describe('fetchDocumentDefs', () => {
    it('populates documentDefsByStep on success', async () => {
      mockListDocumentDefs.mockResolvedValueOnce(mockDefs)
      await workflowStore.fetchDocumentDefs('step-1')

      const state = workflowStore.store.getState()
      expect(state.documentDefsByStep['step-1']).toEqual(mockDefs)
    })

    it('does nothing when no active workflow', async () => {
      workflowStore.store.setState({ activeWorkflowId: null })
      await workflowStore.fetchDocumentDefs('step-1')
      expect(mockListDocumentDefs).not.toHaveBeenCalled()
    })

    it('sets error on failure', async () => {
      mockListDocumentDefs.mockRejectedValueOnce(new Error('Network error'))
      await workflowStore.fetchDocumentDefs('step-1')

      const state = workflowStore.store.getState()
      expect(state.error).toBeTruthy()
    })
  })

  describe('createDocumentDef', () => {
    it('calls API and refetches defs', async () => {
      const newDef: DocumentDef = { ...mockDefs[0]!, id: 'def-new' }
      mockCreateDocumentDef.mockResolvedValueOnce(newDef)
      mockListDocumentDefs.mockResolvedValueOnce([...mockDefs, newDef])

      const result = await workflowStore.createDocumentDef('step-1', {
        name: 'README',
        target_length: 5000,
      })

      expect(result).toEqual(newDef)
      expect(mockCreateDocumentDef).toHaveBeenCalledWith('wf-001', 'step-1', {
        name: 'README',
        target_length: 5000,
      })
      expect(mockListDocumentDefs).toHaveBeenCalled()
    })

    it('returns null when no active workflow', async () => {
      workflowStore.store.setState({ activeWorkflowId: null })
      const result = await workflowStore.createDocumentDef('step-1', {
        name: 'README',
        target_length: 5000,
      })
      expect(result).toBeNull()
      expect(mockCreateDocumentDef).not.toHaveBeenCalled()
    })
  })

  describe('deleteDocumentDef', () => {
    it('calls API and refetches defs', async () => {
      mockDeleteDocumentDef.mockResolvedValueOnce(undefined)
      mockListDocumentDefs.mockResolvedValueOnce([mockDefs[1]!])

      await workflowStore.deleteDocumentDef('step-1', 'def-001')

      expect(mockDeleteDocumentDef).toHaveBeenCalledWith('wf-001', 'step-1', 'def-001')
      expect(mockListDocumentDefs).toHaveBeenCalled()
    })

    it('does nothing when no active workflow', async () => {
      workflowStore.store.setState({ activeWorkflowId: null })
      await workflowStore.deleteDocumentDef('step-1', 'def-001')
      expect(mockDeleteDocumentDef).not.toHaveBeenCalled()
    })
  })
})
