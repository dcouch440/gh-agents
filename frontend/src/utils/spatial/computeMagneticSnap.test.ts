import { describe, it, expect } from 'vitest'
import { computeMagneticSnap } from './computeMagneticSnap'

describe('computeMagneticSnap', () => {
  it('falls back to grid snap when no neighbors within threshold', () => {
    const dragRect = { x: 47, y: 53, width: 50, height: 30 }
    const result = computeMagneticSnap(dragRect, [], 24, 10)
    expect(result.snappedX).toBe(48) // grid-snapped
    expect(result.snappedY).toBe(48) // grid-snapped
    expect(result.activeGuides).toEqual([])
  })

  it('snaps to neighbor left edge when right edge is close', () => {
    const dragRect = { x: 0, y: 0, width: 50, height: 30 }
    const neighbor = { id: 'n', rect: { x: 52, y: 0, width: 50, height: 30 } }
    const result = computeMagneticSnap(dragRect, [neighbor], 24, 10)
    // drag right (50) should snap to neighbor left (52) → snappedX = 52 - 50 = 2
    expect(result.snappedX).toBe(2)
    expect(result.activeGuides.some((g) => g.axis === 'vertical')).toBe(true)
  })

  it('snaps to neighbor right edge when left edge is close', () => {
    const dragRect = { x: 152, y: 0, width: 50, height: 30 }
    const neighbor = { id: 'n', rect: { x: 0, y: 0, width: 150, height: 30 } }
    const result = computeMagneticSnap(dragRect, [neighbor], 24, 10)
    // drag left (152) should snap to neighbor right (150) → snappedX = 150
    expect(result.snappedX).toBe(150)
  })

  it('applies attach gap between snapped edges', () => {
    const dragRect = { x: 0, y: 0, width: 50, height: 30 }
    const neighbor = { id: 'n', rect: { x: 58, y: 0, width: 50, height: 30 } }
    const result = computeMagneticSnap(dragRect, [neighbor], 24, 10, 8)
    // drag right should snap to (neighbor left - gap) = 58 - 8 = 50
    // snappedX = 50 - width = 0
    expect(result.snappedX).toBe(0)
  })

  it('snaps both axes independently', () => {
    const dragRect = { x: 98, y: 48, width: 50, height: 30 }
    const neighbor = { id: 'n', rect: { x: 100, y: 50, width: 50, height: 30 } }
    const result = computeMagneticSnap(dragRect, [neighbor], 24, 10)
    // Left-to-left alignment on both axes
    expect(result.snappedX).toBe(100)
    expect(result.snappedY).toBe(50)
  })
})
