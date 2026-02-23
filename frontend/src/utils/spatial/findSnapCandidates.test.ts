import { describe, it, expect } from 'vitest'
import { findSnapCandidates } from './findSnapCandidates'
import type { AlignmentGuide } from './types'

describe('findSnapCandidates', () => {
  const guides: AlignmentGuide[] = [
    { axis: 'vertical', position: 100, anchorNodeId: 'a' },
    { axis: 'horizontal', position: 200, anchorNodeId: 'a' },
    { axis: 'vertical', position: 500, anchorNodeId: 'b' },
  ]

  it('returns empty when no guides are within threshold', () => {
    const dragRect = { x: 300, y: 300, width: 50, height: 50 }
    expect(findSnapCandidates(dragRect, guides, 5)).toEqual([])
  })

  it('finds vertical guide near left edge', () => {
    const dragRect = { x: 98, y: 300, width: 50, height: 50 }
    const candidates = findSnapCandidates(dragRect, guides, 5)
    expect(candidates.length).toBeGreaterThan(0)
    expect(candidates[0]!.guide.position).toBe(100)
    expect(candidates[0]!.snapEdge).toBe('start')
  })

  it('finds horizontal guide near top edge', () => {
    const dragRect = { x: 300, y: 199, width: 50, height: 50 }
    const candidates = findSnapCandidates(dragRect, guides, 5)
    expect(candidates.length).toBeGreaterThan(0)
    expect(candidates[0]!.guide.axis).toBe('horizontal')
  })

  it('returns candidates sorted by distance ascending', () => {
    const closeGuides: AlignmentGuide[] = [
      { axis: 'vertical', position: 105, anchorNodeId: 'a' },
      { axis: 'vertical', position: 101, anchorNodeId: 'b' },
    ]
    const dragRect = { x: 100, y: 0, width: 50, height: 50 }
    const candidates = findSnapCandidates(dragRect, closeGuides, 10)
    expect(candidates[0]!.distance).toBeLessThanOrEqual(candidates[1]!.distance)
  })
})
