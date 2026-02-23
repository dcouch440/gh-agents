import { describe, it, expect } from 'vitest'
import { buildAlignmentGuides } from './buildAlignmentGuides'

describe('buildAlignmentGuides', () => {
  it('returns empty for empty input', () => {
    expect(buildAlignmentGuides([], new Set())).toEqual([])
  })

  it('emits 6 guides per rectangle', () => {
    const nodes = [{ id: 'a', rect: { x: 0, y: 0, width: 100, height: 50 } }]
    const guides = buildAlignmentGuides(nodes, new Set())
    expect(guides).toHaveLength(6)
  })

  it('emits correct vertical guides (left, right, center-x)', () => {
    const nodes = [{ id: 'a', rect: { x: 10, y: 20, width: 100, height: 50 } }]
    const guides = buildAlignmentGuides(nodes, new Set())
    const verticals = guides.filter((g) => g.axis === 'vertical')
    const positions = verticals.map((g) => g.position).sort((a, b) => a - b)
    expect(positions).toEqual([10, 60, 110]) // left, center, right
  })

  it('emits correct horizontal guides (top, bottom, center-y)', () => {
    const nodes = [{ id: 'a', rect: { x: 10, y: 20, width: 100, height: 50 } }]
    const guides = buildAlignmentGuides(nodes, new Set())
    const horizontals = guides.filter((g) => g.axis === 'horizontal')
    const positions = horizontals.map((g) => g.position).sort((a, b) => a - b)
    expect(positions).toEqual([20, 45, 70]) // top, center, bottom
  })

  it('excludes nodes in excludeIds', () => {
    const nodes = [
      { id: 'a', rect: { x: 0, y: 0, width: 100, height: 50 } },
      { id: 'b', rect: { x: 200, y: 0, width: 100, height: 50 } },
    ]
    const guides = buildAlignmentGuides(nodes, new Set(['a']))
    expect(guides).toHaveLength(6) // only 'b' contributes
    expect(guides.every((g) => g.anchorNodeId === 'b')).toBe(true)
  })

  it('handles multiple rectangles', () => {
    const nodes = [
      { id: 'a', rect: { x: 0, y: 0, width: 100, height: 50 } },
      { id: 'b', rect: { x: 200, y: 0, width: 100, height: 50 } },
    ]
    const guides = buildAlignmentGuides(nodes, new Set())
    expect(guides).toHaveLength(12)
  })
})
