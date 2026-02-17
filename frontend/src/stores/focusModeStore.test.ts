import { focusModeStore } from './focusModeStore'

describe('focusModeStore', () => {
  beforeEach(() => {
    focusModeStore.exit()
  })

  describe('enter', () => {
    it('activates with ordered step IDs and defaults', () => {
      focusModeStore.enter(['s1', 's2', 's3'])
      const state = focusModeStore.store.getState()
      expect(state.active).toBe(true)
      expect(state.orderedStepIds).toEqual(['s1', 's2', 's3'])
      expect(state.currentIndex).toBe(0)
      expect(state.slideDirection).toBe('none')
      expect(state.expandedArtifactId).toBeNull()
      expect(state.activeTabId).toBe('chat')
    })

    it('sets initial index when initialStepId is provided', () => {
      focusModeStore.enter(['s1', 's2', 's3'], 's2')
      const state = focusModeStore.store.getState()
      expect(state.currentIndex).toBe(1)
    })

    it('defaults to index 0 when initialStepId is not found', () => {
      focusModeStore.enter(['s1', 's2'], 'unknown')
      const state = focusModeStore.store.getState()
      expect(state.currentIndex).toBe(0)
    })

    it('does nothing when orderedStepIds is empty', () => {
      focusModeStore.enter([])
      const state = focusModeStore.store.getState()
      expect(state.active).toBe(false)
    })
  })

  describe('exit', () => {
    it('resets to initial state', () => {
      focusModeStore.enter(['s1', 's2'])
      focusModeStore.goNext()
      focusModeStore.expandArtifact('art-1', 'document')
      focusModeStore.exit()

      const state = focusModeStore.store.getState()
      expect(state.active).toBe(false)
      expect(state.orderedStepIds).toEqual([])
      expect(state.currentIndex).toBe(0)
      expect(state.expandedArtifactId).toBeNull()
      expect(state.slideDirection).toBe('none')
    })
  })

  describe('goNext', () => {
    it('increments index and sets slide direction to left', () => {
      focusModeStore.enter(['s1', 's2', 's3'])
      focusModeStore.goNext()
      const state = focusModeStore.store.getState()
      expect(state.currentIndex).toBe(1)
      expect(state.slideDirection).toBe('left')
    })

    it('does not go past the last index', () => {
      focusModeStore.enter(['s1', 's2'])
      focusModeStore.goNext()
      focusModeStore.goNext() // should not go past index 1
      expect(focusModeStore.store.getState().currentIndex).toBe(1)
    })

    it('clears expanded artifact', () => {
      focusModeStore.enter(['s1', 's2'])
      focusModeStore.expandArtifact('art-1', 'document')
      focusModeStore.goNext()
      const state = focusModeStore.store.getState()
      expect(state.expandedArtifactId).toBeNull()
      expect(state.expandedArtifactKind).toBeNull()
    })
  })

  describe('goPrev', () => {
    it('decrements index and sets slide direction to right', () => {
      focusModeStore.enter(['s1', 's2', 's3'])
      focusModeStore.goNext()
      focusModeStore.goPrev()
      const state = focusModeStore.store.getState()
      expect(state.currentIndex).toBe(0)
      expect(state.slideDirection).toBe('right')
    })

    it('does not go below index 0', () => {
      focusModeStore.enter(['s1', 's2'])
      focusModeStore.goPrev() // already at 0
      expect(focusModeStore.store.getState().currentIndex).toBe(0)
    })
  })

  describe('goToIndex', () => {
    it('navigates to a specific index with correct slide direction', () => {
      focusModeStore.enter(['s1', 's2', 's3', 's4'])
      focusModeStore.goToIndex(3)
      expect(focusModeStore.store.getState().currentIndex).toBe(3)
      expect(focusModeStore.store.getState().slideDirection).toBe('left')

      focusModeStore.goToIndex(1)
      expect(focusModeStore.store.getState().currentIndex).toBe(1)
      expect(focusModeStore.store.getState().slideDirection).toBe('right')
    })

    it('does nothing for out-of-bounds index', () => {
      focusModeStore.enter(['s1', 's2'])
      focusModeStore.goToIndex(5)
      expect(focusModeStore.store.getState().currentIndex).toBe(0)
    })

    it('does nothing when navigating to current index', () => {
      focusModeStore.enter(['s1', 's2'])
      focusModeStore.goToIndex(0)
      expect(focusModeStore.store.getState().slideDirection).toBe('none')
    })
  })

  describe('expandArtifact / collapseArtifact', () => {
    it('sets expanded artifact state', () => {
      focusModeStore.enter(['s1'])
      focusModeStore.expandArtifact('doc-1', 'document')
      const state = focusModeStore.store.getState()
      expect(state.expandedArtifactId).toBe('doc-1')
      expect(state.expandedArtifactKind).toBe('document')
    })

    it('clears expanded artifact', () => {
      focusModeStore.enter(['s1'])
      focusModeStore.expandArtifact('doc-1', 'document')
      focusModeStore.collapseArtifact()
      const state = focusModeStore.store.getState()
      expect(state.expandedArtifactId).toBeNull()
      expect(state.expandedArtifactKind).toBeNull()
    })
  })

  describe('setActiveTab', () => {
    it('updates the active tab', () => {
      focusModeStore.enter(['s1'])
      focusModeStore.setActiveTab('debug')
      expect(focusModeStore.store.getState().activeTabId).toBe('debug')
    })
  })

  describe('selectors', () => {
    it('selectCurrentStepId returns correct step or null', () => {
      expect(focusModeStore.selectCurrentStepId(focusModeStore.store.getState())).toBeNull()

      focusModeStore.enter(['s1', 's2'])
      expect(focusModeStore.selectCurrentStepId(focusModeStore.store.getState())).toBe('s1')

      focusModeStore.goNext()
      expect(focusModeStore.selectCurrentStepId(focusModeStore.store.getState())).toBe('s2')
    })

    it('selectStepCount returns the number of steps', () => {
      expect(focusModeStore.selectStepCount(focusModeStore.store.getState())).toBe(0)
      focusModeStore.enter(['s1', 's2', 's3'])
      expect(focusModeStore.selectStepCount(focusModeStore.store.getState())).toBe(3)
    })
  })
})
