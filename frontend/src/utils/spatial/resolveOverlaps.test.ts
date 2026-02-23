import { describe, it, expect } from 'vitest'
import { resolveOverlaps } from './resolveOverlaps'
import type { Overlap } from './types'

describe('resolveOverlaps', () => {
  it('returns empty map when no overlaps', () => {
    const result = resolveOverlaps([], new Map(), 24)
    expect(result.size).toBe(0)
  })

  it('resolves a single overlap by pushing right', () => {
    const overlaps: Overlap[] = [{
      nodeId: 'b',
      overlapRect: { x: 80, y: 0, width: 20, height: 50 },
      pushDirection: 'right',
      pushDistance: 20,
    }]
    const allNodes = new Map([
      ['b', { x: 80, y: 0, width: 100, height: 50 }],
    ])
    const result = resolveOverlaps(overlaps, allNodes, 24)
    expect(result.has('b')).toBe(true)
    const pos = result.get('b')!
    expect(pos.x).toBeGreaterThan(80)
    expect(pos.x % 24).toBe(0) // grid-snapped
  })

  it('resolves a single overlap by pushing left', () => {
    const overlaps: Overlap[] = [{
      nodeId: 'b',
      overlapRect: { x: 0, y: 0, width: 20, height: 50 },
      pushDirection: 'left',
      pushDistance: 20,
    }]
    const allNodes = new Map([
      ['b', { x: 0, y: 0, width: 100, height: 50 }],
    ])
    const result = resolveOverlaps(overlaps, allNodes, 24)
    expect(result.has('b')).toBe(true)
    const pos = result.get('b')!
    expect(pos.x).toBeLessThan(0)
  })

  it('respects maxDepth limit', () => {
    // Create a chain that would cascade indefinitely
    const overlaps: Overlap[] = [{
      nodeId: 'b',
      overlapRect: { x: 90, y: 0, width: 10, height: 50 },
      pushDirection: 'right',
      pushDistance: 10,
    }]
    const allNodes = new Map([
      ['b', { x: 90, y: 0, width: 50, height: 50 }],
      ['c', { x: 130, y: 0, width: 50, height: 50 }],
    ])
    const result = resolveOverlaps(overlaps, allNodes, 24, 1)
    // Should resolve b but may not cascade to c due to depth limit
    expect(result.has('b')).toBe(true)
  })
})
