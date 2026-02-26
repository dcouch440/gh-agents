import { describe, it, expect, beforeEach } from 'vitest'
import { sidebarStore } from './sidebarStore'

describe('sidebarStore', () => {
  beforeEach(() => {
    sidebarStore.reset()
  })

  it('has correct initial state', () => {
    const s = sidebarStore.store.getState()
    expect(s.activeTab).toBe('tree')
    expect(s.selectedStepId).toBe(null)
    expect(s.dragging).toBe(false)
    expect(s.width).toBeGreaterThanOrEqual(sidebarStore.MIN_WIDTH)
    expect(s.width).toBeLessThanOrEqual(sidebarStore.MAX_WIDTH)
  })

  it('setActiveTab switches tabs', () => {
    sidebarStore.setActiveTab('chat')
    expect(sidebarStore.store.getState().activeTab).toBe('chat')

    sidebarStore.setActiveTab('tree')
    expect(sidebarStore.store.getState().activeTab).toBe('tree')
  })

  it('selectStep sets selectedStepId', () => {
    sidebarStore.selectStep('step-123')
    expect(sidebarStore.store.getState().selectedStepId).toBe('step-123')
  })

  it('selectStep switches to tree tab when on chat', () => {
    sidebarStore.setActiveTab('chat')
    sidebarStore.selectStep('step-456')

    const s = sidebarStore.store.getState()
    expect(s.selectedStepId).toBe('step-456')
    expect(s.activeTab).toBe('tree')
  })

  it('clearSelection sets selectedStepId to null', () => {
    sidebarStore.selectStep('step-123')
    sidebarStore.clearSelection()
    expect(sidebarStore.store.getState().selectedStepId).toBe(null)
  })

  it('setWidth clamps to min', () => {
    sidebarStore.setWidth(100)
    expect(sidebarStore.store.getState().width).toBe(sidebarStore.MIN_WIDTH)
  })

  it('setWidth clamps to max', () => {
    sidebarStore.setWidth(9999)
    expect(sidebarStore.store.getState().width).toBe(sidebarStore.MAX_WIDTH)
  })

  it('setWidth accepts valid width', () => {
    sidebarStore.setWidth(350)
    expect(sidebarStore.store.getState().width).toBe(350)
  })

  it('startDrag / stopDrag toggles dragging', () => {
    sidebarStore.startDrag()
    expect(sidebarStore.store.getState().dragging).toBe(true)

    sidebarStore.stopDrag()
    expect(sidebarStore.store.getState().dragging).toBe(false)
  })

  it('reset clears selection and dragging, keeps width', () => {
    sidebarStore.selectStep('step-789')
    sidebarStore.setActiveTab('chat')
    sidebarStore.startDrag()
    sidebarStore.setWidth(400)

    sidebarStore.reset()

    const s = sidebarStore.store.getState()
    expect(s.activeTab).toBe('tree')
    expect(s.selectedStepId).toBe(null)
    expect(s.dragging).toBe(false)
    // Width is preserved (not reset)
    expect(s.width).toBe(400)
  })

  // ── Expand / collapse ──────────────────────────────────────────────────────

  it('toggleStep toggles expand state', () => {
    sidebarStore.toggleStep('s-1')
    expect(sidebarStore.store.getState().expandedStepIds['s-1']).toBe(true)

    sidebarStore.toggleStep('s-1')
    expect(sidebarStore.store.getState().expandedStepIds['s-1']).toBe(false)
  })

  it('expandStep sets step expanded', () => {
    sidebarStore.expandStep('s-2')
    expect(sidebarStore.store.getState().expandedStepIds['s-2']).toBe(true)
  })

  it('expandStep is idempotent', () => {
    sidebarStore.expandStep('s-2')
    sidebarStore.expandStep('s-2')
    expect(sidebarStore.store.getState().expandedStepIds['s-2']).toBe(true)
  })

  it('collapseStep sets step collapsed', () => {
    sidebarStore.expandStep('s-3')
    sidebarStore.collapseStep('s-3')
    expect(sidebarStore.store.getState().expandedStepIds['s-3']).toBe(false)
  })

  it('multiple steps can be expanded independently', () => {
    sidebarStore.expandStep('s-1')
    sidebarStore.expandStep('s-2')
    const s = sidebarStore.store.getState()
    expect(s.expandedStepIds['s-1']).toBe(true)
    expect(s.expandedStepIds['s-2']).toBe(true)
  })

  it('toggleOutputExpand toggles output expand state', () => {
    sidebarStore.toggleOutputExpand('s-1')
    expect(sidebarStore.store.getState().outputExpandedStepIds['s-1']).toBe(true)

    sidebarStore.toggleOutputExpand('s-1')
    expect(sidebarStore.store.getState().outputExpandedStepIds['s-1']).toBe(false)
  })

  it('reset clears expand state', () => {
    sidebarStore.expandStep('s-1')
    sidebarStore.toggleOutputExpand('s-2')
    sidebarStore.reset()

    const s = sidebarStore.store.getState()
    expect(s.expandedStepIds).toEqual({})
    expect(s.outputExpandedStepIds).toEqual({})
  })

  // ── Selectors ─────────────────────────────────────────────────────────────

  it('selectIsExpanded returns correct value', () => {
    sidebarStore.expandStep('s-1')
    const s = sidebarStore.store.getState()
    expect(sidebarStore.selectIsExpanded('s-1')(s)).toBe(true)
    expect(sidebarStore.selectIsExpanded('s-2')(s)).toBe(false)
  })

  it('selectIsOutputExpanded returns correct value', () => {
    sidebarStore.toggleOutputExpand('s-1')
    const s = sidebarStore.store.getState()
    expect(sidebarStore.selectIsOutputExpanded('s-1')(s)).toBe(true)
    expect(sidebarStore.selectIsOutputExpanded('s-2')(s)).toBe(false)
  })

  it('selectors return correct values', () => {
    sidebarStore.selectStep('s-1')
    sidebarStore.setActiveTab('tree')
    sidebarStore.setWidth(300)
    sidebarStore.startDrag()

    const s = sidebarStore.store.getState()
    expect(sidebarStore.selectActiveTab(s)).toBe('tree')
    expect(sidebarStore.selectSelectedStepId(s)).toBe('s-1')
    expect(sidebarStore.selectWidth(s)).toBe(300)
    expect(sidebarStore.selectDragging(s)).toBe(true)
  })
})
