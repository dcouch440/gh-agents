import { describe, it, expect } from 'vitest'
import { detectOverlaps } from './detectOverlaps'

describe('detectOverlaps', () => {
  it('returns empty when no overlaps exist', () => {
    const others = [{ id: 'b', rect: { x: 200, y: 0, width: 50, height: 50 } }]
    const result = detectOverlaps({ x: 0, y: 0, width: 50, height: 50 }, 'a', others)
    expect(result).toEqual([])
  })

  it('detects an overlap and computes push direction', () => {
    const movedRect = { x: 0, y: 0, width: 100, height: 50 }
    const others = [{ id: 'b', rect: { x: 80, y: 0, width: 100, height: 50 } }]
    const result = detectOverlaps(movedRect, 'a', others)
    expect(result).toHaveLength(1)
    expect(result[0]!.nodeId).toBe('b')
    expect(result[0]!.pushDirection).toBe('right')
    expect(result[0]!.pushDistance).toBe(20) // 20px overlap on x-axis
  })

  it('skips self (movedId)', () => {
    const movedRect = { x: 0, y: 0, width: 100, height: 50 }
    const others = [{ id: 'a', rect: { x: 0, y: 0, width: 100, height: 50 } }]
    const result = detectOverlaps(movedRect, 'a', others)
    expect(result).toEqual([])
  })

  it('pushes vertically when height overlap is smaller', () => {
    const movedRect = { x: 0, y: 0, width: 50, height: 100 }
    const others = [{ id: 'b', rect: { x: 0, y: 90, width: 50, height: 100 } }]
    const result = detectOverlaps(movedRect, 'a', others)
    expect(result).toHaveLength(1)
    expect(result[0]!.pushDirection).toBe('bottom')
    expect(result[0]!.pushDistance).toBe(10)
  })

  it('detects multiple overlaps', () => {
    const movedRect = { x: 50, y: 50, width: 100, height: 100 }
    const others = [
      { id: 'b', rect: { x: 0, y: 0, width: 80, height: 80 } },
      { id: 'c', rect: { x: 120, y: 120, width: 80, height: 80 } },
    ]
    const result = detectOverlaps(movedRect, 'a', others)
    expect(result).toHaveLength(2)
  })
})
