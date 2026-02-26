import { describe, expect, it } from 'vitest'
import type { BoxElement } from '../elements'
import { computeGeometricAnchor, computeBindingAnchor } from './binding'

// ============================================================================
// Helpers
// ============================================================================

const makeBox = (x: number, y: number, width: number, height: number): BoxElement => ({
  id: 'test-box',
  type: 'box',
  x,
  y,
  width,
  height,
  text: '',
})

// ============================================================================
// computeGeometricAnchor
// ============================================================================

describe('computeGeometricAnchor', () => {
  const box = makeBox(100, 100, 200, 100)

  it('picks left side when source is to the left', () => {
    const anchor = computeGeometricAnchor(box, { x: 0, y: 150 })
    expect(anchor.side).toBe('left')
    expect(anchor.ratio).toBe(0.5)
  })

  it('picks right side when source is to the right', () => {
    const anchor = computeGeometricAnchor(box, { x: 500, y: 150 })
    expect(anchor.side).toBe('right')
    expect(anchor.ratio).toBe(0.5)
  })

  it('picks top side when source is above', () => {
    const anchor = computeGeometricAnchor(box, { x: 200, y: 0 })
    expect(anchor.side).toBe('top')
    expect(anchor.ratio).toBe(0.5)
  })

  it('picks bottom side when source is below', () => {
    const anchor = computeGeometricAnchor(box, { x: 200, y: 400 })
    expect(anchor.side).toBe('bottom')
    expect(anchor.ratio).toBe(0.5)
  })

  it('prefers left/right for wide boxes when source is clearly lateral', () => {
    const wideBox = makeBox(100, 100, 400, 100)
    // Source far to the left, vertically centered — should pick left
    const anchor = computeGeometricAnchor(wideBox, { x: 0, y: 150 })
    expect(anchor.side).toBe('left')
  })

  it('prefers top/bottom for tall boxes when source is clearly above/below', () => {
    const tallBox = makeBox(100, 100, 100, 400)
    // Source far above, horizontally centered — should pick top
    const anchor = computeGeometricAnchor(tallBox, { x: 150, y: 0 })
    expect(anchor.side).toBe('top')
  })

  it('always returns ratio 0.5', () => {
    const anchor1 = computeGeometricAnchor(box, { x: 0, y: 0 })
    const anchor2 = computeGeometricAnchor(box, { x: 500, y: 500 })
    const anchor3 = computeGeometricAnchor(box, { x: 200, y: 0 })

    expect(anchor1.ratio).toBe(0.5)
    expect(anchor2.ratio).toBe(0.5)
    expect(anchor3.ratio).toBe(0.5)
  })
})

// ============================================================================
// computeBindingAnchor (existing, verify still works)
// ============================================================================

describe('computeBindingAnchor', () => {
  const box = makeBox(100, 100, 200, 100)

  it('snaps to midpoint when cursor is near side center', () => {
    // Near the center of the top side
    const anchor = computeBindingAnchor(box, { x: 200, y: 95 })
    expect(anchor.side).toBe('top')
    expect(anchor.ratio).toBe(0.5)
  })

  it('picks the nearest side', () => {
    // Near the right side
    const anchor = computeBindingAnchor(box, { x: 305, y: 150 })
    expect(anchor.side).toBe('right')
  })

  it('clamps ratio to avoid corners', () => {
    // Near the top side, slightly off-center
    const anchor = computeBindingAnchor(box, { x: 110, y: 98 })
    expect(anchor.ratio).toBeGreaterThanOrEqual(0.1)
    expect(anchor.ratio).toBeLessThanOrEqual(0.9)
  })
})
