import { describe, it, expect, vi, beforeEach } from 'vitest'
import type { DocumentDef } from '@/types/workflow'

const {
  mockListDocumentDefs,
  mockCreateDocumentDef,
  mockDeleteDocumentDef,
  mockGetDocument,
} = vi.hoisted(() => ({
  mockListDocumentDefs: vi.fn(),
  mockCreateDocumentDef: vi.fn(),
  mockDeleteDocumentDef: vi.fn(),
  mockGetDocument: vi.fn(),
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
    documents: {
      get: mockGetDocument,
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
    document_id: null,
    agent_roster_entry_id: null,
  },
  {
    id: 'def-002',
    step_id: 'step-1',
    name: 'CHANGELOG',
    description: null,
    target_length: 2000,
    display_order: 1,
    created_at: '2025-01-01T00:00:00Z',
    document_id: null,
    agent_roster_entry_id: null,
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

  describe('fetchDocumentContent', () => {
    const defsWithDocId: DocumentDef[] = [
      { ...mockDefs[0]!, document_id: 'doc-aaa' },
      { ...mockDefs[1]!, document_id: 'doc-bbb' },
    ]

    it('fetches content for defs that have a document_id', async () => {
      workflowStore.store.setState({ documentDefsByStep: { 'step-1': defsWithDocId } })
      mockGetDocument
        .mockResolvedValueOnce({ id: 'doc-aaa', content: '# README Content' })
        .mockResolvedValueOnce({ id: 'doc-bbb', content: '# CHANGELOG Content' })

      await workflowStore.fetchDocumentContent('step-1')

      expect(mockGetDocument).toHaveBeenCalledTimes(2)
      expect(mockGetDocument).toHaveBeenCalledWith('doc-aaa')
      expect(mockGetDocument).toHaveBeenCalledWith('doc-bbb')

      const state = workflowStore.store.getState()
      expect(state.documentContentByDefId['def-001']).toBe('# README Content')
      expect(state.documentContentByDefId['def-002']).toBe('# CHANGELOG Content')
    })

    it('skips defs without a document_id', async () => {
      // mockDefs have document_id: null
      workflowStore.store.setState({ documentDefsByStep: { 'step-1': mockDefs } })

      await workflowStore.fetchDocumentContent('step-1')

      expect(mockGetDocument).not.toHaveBeenCalled()
    })

    it('does nothing when no defs exist for step', async () => {
      workflowStore.store.setState({ documentDefsByStep: {} })

      await workflowStore.fetchDocumentContent('step-1')

      expect(mockGetDocument).not.toHaveBeenCalled()
    })

    it('preserves existing content for other defs', async () => {
      workflowStore.store.setState({
        documentDefsByStep: { 'step-1': [defsWithDocId[0]!] },
        documentContentByDefId: { 'def-existing': 'keep me' },
      })
      mockGetDocument.mockResolvedValueOnce({ id: 'doc-aaa', content: 'new content' })

      await workflowStore.fetchDocumentContent('step-1')

      const state = workflowStore.store.getState()
      expect(state.documentContentByDefId['def-001']).toBe('new content')
      expect(state.documentContentByDefId['def-existing']).toBe('keep me')
    })

    it('silently catches fetch errors', async () => {
      workflowStore.store.setState({
        documentDefsByStep: { 'step-1': defsWithDocId },
        documentContentByDefId: {},
      })
      mockGetDocument.mockRejectedValueOnce(new Error('Network error'))

      // Should not throw
      await workflowStore.fetchDocumentContent('step-1')

      // No new content stored, no error set
      const state = workflowStore.store.getState()
      expect(state.documentContentByDefId).toEqual({})
    })
  })
})
