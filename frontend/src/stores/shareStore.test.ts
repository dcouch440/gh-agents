import { shareStore } from './shareStore'
import type { PickableEntity } from './contextMentionStore'

const { mockAddMention } = vi.hoisted(() => ({
  mockAddMention: vi.fn(),
}))

vi.mock('./contextMentionStore', () => ({
  contextMentionStore: {
    addMention: mockAddMention,
  },
}))

const entity1: PickableEntity = {
  kind: 'agent',
  id: 'e1',
  name: 'Agent Alpha',
  summary: 'A test agent',
  data: {},
}

const entity2: PickableEntity = {
  kind: 'document',
  id: 'e2',
  name: 'Doc Beta',
  summary: 'A test doc',
  data: {},
}

const field1 = {
  key: 'f1',
  label: 'Field 1',
  category: 'agents',
  kind: 'agent' as const,
  entity: entity1,
  color: '#ff0000',
  chipKey: 'agent:e1',
}

const field2 = {
  key: 'f2',
  label: 'Field 2',
  category: 'documents',
  kind: 'document' as const,
  entity: entity2,
  color: '#00ff00',
  chipKey: 'doc:e2',
}

beforeEach(() => {
  vi.clearAllMocks()
  shareStore.cancelShare()
})

describe('shareStore', () => {
  describe('enterShareMode', () => {
    it('activates share mode with all fields selected', () => {
      shareStore.enterShareMode('step-1', [field1, field2])

      const state = shareStore.store.getState()
      expect(state.active).toBe(true)
      expect(state.sourceStepId).toBe('step-1')
      expect(state.availableFields).toEqual([field1, field2])
      expect(state.selectedKeys.has('f1')).toBe(true)
      expect(state.selectedKeys.has('f2')).toBe(true)
      expect(state.selectedKeys.size).toBe(2)
    })

    it('clears pending chat focus', () => {
      shareStore.enterShareMode('step-1', [field1])
      expect(shareStore.store.getState().pendingChatFocus).toBeNull()
    })
  })

  describe('toggleField', () => {
    it('removes a selected field', () => {
      shareStore.enterShareMode('step-1', [field1, field2])

      shareStore.toggleField('f1')

      const keys = shareStore.store.getState().selectedKeys
      expect(keys.has('f1')).toBe(false)
      expect(keys.has('f2')).toBe(true)
    })

    it('adds back a deselected field', () => {
      shareStore.enterShareMode('step-1', [field1, field2])
      shareStore.toggleField('f1') // deselect
      shareStore.toggleField('f1') // re-select

      expect(shareStore.store.getState().selectedKeys.has('f1')).toBe(true)
    })
  })

  describe('commitShare', () => {
    it('calls addMention for each selected field', () => {
      shareStore.enterShareMode('step-src', [field1, field2])

      shareStore.commitShare('step-target')

      expect(mockAddMention).toHaveBeenCalledTimes(2)
      expect(mockAddMention).toHaveBeenCalledWith('step-target', entity1, '#ff0000', expect.objectContaining({
        chipKey: 'agent:e1',
      }))
      expect(mockAddMention).toHaveBeenCalledWith('step-target', entity2, '#00ff00', expect.objectContaining({
        chipKey: 'doc:e2',
      }))
    })

    it('only commits selected fields', () => {
      shareStore.enterShareMode('step-src', [field1, field2])
      shareStore.toggleField('f2') // deselect field2

      shareStore.commitShare('step-target')

      expect(mockAddMention).toHaveBeenCalledTimes(1)
      expect(mockAddMention).toHaveBeenCalledWith('step-target', entity1, '#ff0000', expect.any(Object))
    })

    it('is a no-op when source equals target', () => {
      shareStore.enterShareMode('step-1', [field1])

      shareStore.commitShare('step-1')

      expect(mockAddMention).not.toHaveBeenCalled()
    })

    it('is a no-op when not active', () => {
      shareStore.commitShare('step-target')
      expect(mockAddMention).not.toHaveBeenCalled()
    })

    it('resets state and sets pending chat focus', () => {
      shareStore.enterShareMode('step-src', [field1])

      shareStore.commitShare('step-target')

      const state = shareStore.store.getState()
      expect(state.active).toBe(false)
      expect(state.sourceStepId).toBeNull()
      expect(state.pendingChatFocus).toBe('step-target')
    })
  })

  describe('cancelShare', () => {
    it('resets all state', () => {
      shareStore.enterShareMode('step-1', [field1, field2])

      shareStore.cancelShare()

      const state = shareStore.store.getState()
      expect(state.active).toBe(false)
      expect(state.sourceStepId).toBeNull()
      expect(state.availableFields).toEqual([])
      expect(state.selectedKeys.size).toBe(0)
      expect(state.pendingChatFocus).toBeNull()
    })
  })

  describe('clearPendingChatFocus', () => {
    it('clears the pending focus', () => {
      shareStore.enterShareMode('step-src', [field1])
      shareStore.commitShare('step-target')
      expect(shareStore.store.getState().pendingChatFocus).toBe('step-target')

      shareStore.clearPendingChatFocus()

      expect(shareStore.store.getState().pendingChatFocus).toBeNull()
    })
  })

  describe('selectors', () => {
    it('selectAvailableFields returns empty when not active', () => {
      expect(shareStore.selectAvailableFields(shareStore.store.getState())).toEqual([])
    })

    it('selectSelectedKeys returns empty set when not active', () => {
      expect(shareStore.selectSelectedKeys(shareStore.store.getState()).size).toBe(0)
    })

    it('selectAvailableFields returns fields when active', () => {
      shareStore.enterShareMode('step-1', [field1])
      expect(shareStore.selectAvailableFields(shareStore.store.getState())).toEqual([field1])
    })
  })
})
