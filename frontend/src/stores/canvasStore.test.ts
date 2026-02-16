import { canvasStore } from './canvasStore'
import type { CanvasState } from './canvasStore'

const getState = (): CanvasState => canvasStore.store.getState()

beforeEach(() => {
  canvasStore.reset()
})

describe('canvasStore', () => {
  describe('selection', () => {
    it('selectSteps replaces step selection', () => {
      canvasStore.selectSteps(['s1', 's2'])

      const ids = getState().selectedStepIds
      expect(ids.size).toBe(2)
      expect(ids.has('s1')).toBe(true)
      expect(ids.has('s2')).toBe(true)
    })

    it('selectEdges replaces edge selection', () => {
      canvasStore.selectEdges(['e1'])

      const ids = getState().selectedEdgeIds
      expect(ids.size).toBe(1)
      expect(ids.has('e1')).toBe(true)
    })

    it('addToSelection toggles step in selection', () => {
      canvasStore.addToSelection('s1')
      expect(getState().selectedStepIds.has('s1')).toBe(true)

      canvasStore.addToSelection('s1')
      expect(getState().selectedStepIds.has('s1')).toBe(false)
    })

    it('clearSelection empties both sets', () => {
      canvasStore.selectSteps(['s1', 's2'])
      canvasStore.selectEdges(['e1'])

      canvasStore.clearSelection()

      expect(getState().selectedStepIds.size).toBe(0)
      expect(getState().selectedEdgeIds.size).toBe(0)
    })

    it('selectHasSelection returns true when steps selected', () => {
      canvasStore.selectSteps(['s1'])
      expect(canvasStore.selectHasSelection(getState())).toBe(true)
    })

    it('selectHasSelection returns false when empty', () => {
      expect(canvasStore.selectHasSelection(getState())).toBe(false)
    })
  })

  describe('panel', () => {
    it('openPanel sets kind and targetId', () => {
      canvasStore.openPanel('step-config', 's1')

      expect(getState().panel).toBe('step-config')
      expect(getState().panelTargetId).toBe('s1')
    })

    it('openPanel replaces previous panel', () => {
      canvasStore.openPanel('step-config', 's1')
      canvasStore.openPanel('edge-config', 'e1')

      expect(getState().panel).toBe('edge-config')
      expect(getState().panelTargetId).toBe('e1')
    })

    it('closePanel resets to closed', () => {
      canvasStore.openPanel('step-config', 's1')
      canvasStore.closePanel()

      expect(getState().panel).toBe('closed')
      expect(getState().panelTargetId).toBeNull()
    })
  })

  describe('interaction mode', () => {
    it('setInteractionMode changes mode', () => {
      canvasStore.setInteractionMode('connect')
      expect(getState().interactionMode).toBe('connect')

      canvasStore.setInteractionMode('pan')
      expect(getState().interactionMode).toBe('pan')
    })

    it('defaults to select mode', () => {
      expect(getState().interactionMode).toBe('select')
    })
  })

  describe('hover', () => {
    it('setHoveredStep sets and clears', () => {
      canvasStore.setHoveredStep('s1')
      expect(getState().hoveredStepId).toBe('s1')

      canvasStore.setHoveredStep(null)
      expect(getState().hoveredStepId).toBeNull()
    })

    it('setHoveredEdge sets and clears', () => {
      canvasStore.setHoveredEdge('e1')
      expect(getState().hoveredEdgeId).toBe('e1')

      canvasStore.setHoveredEdge(null)
      expect(getState().hoveredEdgeId).toBeNull()
    })
  })

  describe('drag', () => {
    it('setDragItem sets and clears', () => {
      const item = { type: 'step' as const, stepId: 's1', startX: 10, startY: 20 }
      canvasStore.setDragItem(item)
      expect(getState().dragItem).toEqual(item)

      canvasStore.setDragItem(null)
      expect(getState().dragItem).toBeNull()
    })
  })

  describe('minimap', () => {
    it('toggleMinimap flips visibility', () => {
      expect(getState().minimapVisible).toBe(false)

      canvasStore.toggleMinimap()
      expect(getState().minimapVisible).toBe(true)

      canvasStore.toggleMinimap()
      expect(getState().minimapVisible).toBe(false)
    })
  })

  describe('reset', () => {
    it('clears all state to defaults', () => {
      canvasStore.selectSteps(['s1', 's2'])
      canvasStore.selectEdges(['e1'])
      canvasStore.setHoveredStep('s1')
      canvasStore.setHoveredEdge('e1')
      canvasStore.openPanel('step-config', 's1')
      canvasStore.setInteractionMode('connect')
      canvasStore.setDragItem({ type: 'step', stepId: 's1', startX: 0, startY: 0 })
      canvasStore.toggleMinimap()

      canvasStore.reset()

      const s = getState()
      expect(s.selectedStepIds.size).toBe(0)
      expect(s.selectedEdgeIds.size).toBe(0)
      expect(s.hoveredStepId).toBeNull()
      expect(s.hoveredEdgeId).toBeNull()
      expect(s.panel).toBe('closed')
      expect(s.panelTargetId).toBeNull()
      expect(s.interactionMode).toBe('select')
      expect(s.dragItem).toBeNull()
      expect(s.minimapVisible).toBe(false)
    })
  })

  describe('step protocols', () => {
    it('linkStepProtocol adds a protocol link', () => {
      const link = {
        protocolId: 'proto-1',
        protocolType: 'workforce',
        protocolName: 'Doc Writer',
        portNames: ['input'],
      }

      canvasStore.linkStepProtocol('step-1', link)

      expect(getState().stepProtocols['step-1']).toEqual(link)
    })

    it('linkStepProtocol overwrites existing link for same step', () => {
      const link1 = { protocolId: 'p1', protocolType: 'workforce', protocolName: 'First', portNames: [] }
      const link2 = { protocolId: 'p2', protocolType: 'workforce', protocolName: 'Second', portNames: ['a'] }

      canvasStore.linkStepProtocol('step-1', link1)
      canvasStore.linkStepProtocol('step-1', link2)

      expect(getState().stepProtocols['step-1']).toEqual(link2)
    })

    it('unlinkStepProtocol removes a protocol link', () => {
      const link = { protocolId: 'p1', protocolType: 'workforce', protocolName: 'Doc', portNames: [] }
      canvasStore.linkStepProtocol('step-1', link)

      canvasStore.unlinkStepProtocol('step-1')

      expect(getState().stepProtocols['step-1']).toBeUndefined()
    })

    it('unlinkStepProtocol is a no-op for missing step', () => {
      canvasStore.unlinkStepProtocol('nonexistent')
      expect(Object.keys(getState().stepProtocols)).toHaveLength(0)
    })

    it('selectStepProtocols returns current protocols', () => {
      const link = { protocolId: 'p1', protocolType: 'workforce', protocolName: 'Doc', portNames: [] }
      canvasStore.linkStepProtocol('step-1', link)

      expect(canvasStore.selectStepProtocols(getState())).toEqual({ 'step-1': link })
    })

    it('reset clears stepProtocols', () => {
      canvasStore.linkStepProtocol('step-1', {
        protocolId: 'p1',
        protocolType: 'workforce',
        protocolName: 'Doc',
        portNames: [],
      })

      canvasStore.reset()

      expect(Object.keys(getState().stepProtocols)).toHaveLength(0)
    })
  })

  describe('highlighted protocols', () => {
    it('setHighlightedProtocols updates the set', () => {
      const ids = new Set(['s1', 's2'])
      canvasStore.setHighlightedProtocols(ids)

      expect(getState().highlightedProtocolStepIds).toEqual(ids)
    })

    it('setHighlightedProtocols skips update when equal', () => {
      const ids = new Set(['s1'])
      canvasStore.setHighlightedProtocols(ids)
      const stateBefore = getState()

      canvasStore.setHighlightedProtocols(new Set(['s1']))
      const stateAfter = getState()

      // Same reference means no update occurred
      expect(stateBefore.highlightedProtocolStepIds).toBe(stateAfter.highlightedProtocolStepIds)
    })
  })

  describe('hover with protocol', () => {
    it('setHoveredStep stores protocolId when provided', () => {
      canvasStore.setHoveredStep('s1', 'proto-1')

      expect(getState().hoveredStepId).toBe('s1')
      expect(getState().hoveredProtocolId).toBe('proto-1')
    })

    it('setHoveredStep clears protocolId when not provided', () => {
      canvasStore.setHoveredStep('s1', 'proto-1')
      canvasStore.setHoveredStep('s2')

      expect(getState().hoveredStepId).toBe('s2')
      expect(getState().hoveredProtocolId).toBeNull()
    })
  })

  describe('selectors', () => {
    it('selectSelectedStepIds returns current set', () => {
      canvasStore.selectSteps(['s1'])
      expect(canvasStore.selectSelectedStepIds(getState()).has('s1')).toBe(true)
    })

    it('selectPanel returns current panel kind', () => {
      expect(canvasStore.selectPanel(getState())).toBe('closed')
    })

    it('selectMinimapVisible returns current value', () => {
      expect(canvasStore.selectMinimapVisible(getState())).toBe(false)
    })
  })
})
