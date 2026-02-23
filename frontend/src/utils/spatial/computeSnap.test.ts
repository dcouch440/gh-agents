import { describe, it, expect } from 'vitest'
import { computeSnap } from './computeSnap'
import type { SnapCandidate } from './types'

describe('computeSnap', () => {
  const dragRect = { x: 100, y: 200, width: 50, height: 30 }

  it('returns original position when no candidates', () => {
    const result = computeSnap(dragRect, [])
    expect(result.snappedX).toBe(100)
    expect(result.snappedY).toBe(200)
    expect(result.activeGuides).toEqual([])
  })

  it('snaps x to vertical guide start edge', () => {
    const candidates: SnapCandidate[] = [{
      guide: { axis: 'vertical', position: 105, anchorNodeId: 'a' },
      distance: 5,
      snapEdge: 'start',
    }]
    const result = computeSnap(dragRect, candidates)
    expect(result.snappedX).toBe(105)
    expect(result.snappedY).toBe(200) // unchanged
  })

  it('snaps x to vertical guide end edge', () => {
    const candidates: SnapCandidate[] = [{
      guide: { axis: 'vertical', position: 155, anchorNodeId: 'a' },
      distance: 5,
      snapEdge: 'end',
    }]
    const result = computeSnap(dragRect, candidates)
    expect(result.snappedX).toBe(105) // 155 - width(50)
  })

  it('snaps x to vertical guide center edge', () => {
    const candidates: SnapCandidate[] = [{
      guide: { axis: 'vertical', position: 130, anchorNodeId: 'a' },
      distance: 5,
      snapEdge: 'center',
    }]
    const result = computeSnap(dragRect, candidates)
    expect(result.snappedX).toBe(105) // 130 - width/2(25)
  })

  it('snaps y to horizontal guide', () => {
    const candidates: SnapCandidate[] = [{
      guide: { axis: 'horizontal', position: 210, anchorNodeId: 'a' },
      distance: 10,
      snapEdge: 'start',
    }]
    const result = computeSnap(dragRect, candidates)
    expect(result.snappedY).toBe(210)
  })

  it('picks best candidate per axis independently', () => {
    const candidates: SnapCandidate[] = [
      { guide: { axis: 'vertical', position: 102, anchorNodeId: 'a' }, distance: 2, snapEdge: 'start' },
      { guide: { axis: 'horizontal', position: 203, anchorNodeId: 'b' }, distance: 3, snapEdge: 'start' },
      { guide: { axis: 'vertical', position: 108, anchorNodeId: 'c' }, distance: 8, snapEdge: 'start' },
    ]
    const result = computeSnap(dragRect, candidates)
    expect(result.snappedX).toBe(102) // best vertical
    expect(result.snappedY).toBe(203) // best horizontal
    expect(result.activeGuides).toHaveLength(2)
  })
})
