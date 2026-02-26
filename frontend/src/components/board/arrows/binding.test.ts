import { describe, expect, it } from 'vitest'
import type { BoxElement } from '../elements'
import {
  anchorToFocus,
  computeBindingAnchor,
  computeGeometricFocus,
  focusToAbsolute,
  focusToPerimeter,
  rayBoxIntersection,
} from './binding'

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
// rayBoxIntersection
// ============================================================================

describe('rayBoxIntersection', () => {
  const box = makeBox(100, 100, 200, 100)

  it('exits right when aiming right from center', () => {
    const center = { x: 200, y: 150 }
    const target = { x: 500, y: 150 }
    const result = rayBoxIntersection(box, center, target)
    expect(result.side).toBe('right')
    expect(result.point.x).toBe(300)
    expect(result.point.y).toBe(150)
  })

  it('exits left when aiming left from center', () => {
    const center = { x: 200, y: 150 }
    const target = { x: 0, y: 150 }
    const result = rayBoxIntersection(box, center, target)
    expect(result.side).toBe('left')
    expect(result.point.x).toBe(100)
    expect(result.point.y).toBe(150)
  })

  it('exits top when aiming up from center', () => {
    const center = { x: 200, y: 150 }
    const target = { x: 200, y: 0 }
    const result = rayBoxIntersection(box, center, target)
    expect(result.side).toBe('top')
    expect(result.point.y).toBe(100)
  })

  it('exits bottom when aiming down from center', () => {
    const center = { x: 200, y: 150 }
    const target = { x: 200, y: 400 }
    const result = rayBoxIntersection(box, center, target)
    expect(result.side).toBe('bottom')
    expect(result.point.y).toBe(200)
  })

  it('exits correct side from off-center focus', () => {
    // Focus at top-right area of box, aiming further right
    const focus = { x: 280, y: 110 }
    const target = { x: 500, y: 110 }
    const result = rayBoxIntersection(box, focus, target)
    expect(result.side).toBe('right')
    expect(result.point.x).toBe(300)
  })

  it('exits at diagonal through the correct side', () => {
    const center = { x: 200, y: 150 }
    // Aiming up-right — top edge is 50px away vertically, right edge is 100px away
    // horizontally. At 45 degrees (dx=200, dy=-200), top is hit first.
    const target = { x: 400, y: -50 }
    const result = rayBoxIntersection(box, center, target)
    expect(result.side).toBe('top')
  })

  it('handles zero-length ray with fallback', () => {
    const point = { x: 200, y: 150 }
    const result = rayBoxIntersection(box, point, point)
    expect(result.side).toBe('right')
  })
})

// ============================================================================
// focusToAbsolute
// ============================================================================

describe('focusToAbsolute', () => {
  const box = makeBox(100, 100, 200, 100)

  it('converts center focus to box center', () => {
    const point = focusToAbsolute(box, { fx: 0.5, fy: 0.5 })
    expect(point.x).toBe(200)
    expect(point.y).toBe(150)
  })

  it('converts top-left focus to box top-left', () => {
    const point = focusToAbsolute(box, { fx: 0, fy: 0 })
    expect(point.x).toBe(100)
    expect(point.y).toBe(100)
  })

  it('converts bottom-right focus to box bottom-right', () => {
    const point = focusToAbsolute(box, { fx: 1, fy: 1 })
    expect(point.x).toBe(300)
    expect(point.y).toBe(200)
  })
})

// ============================================================================
// focusToPerimeter
// ============================================================================

describe('focusToPerimeter', () => {
  const box = makeBox(100, 100, 200, 100)

  it('computes perimeter point from center focus aiming right', () => {
    const result = focusToPerimeter(box, { fx: 0.5, fy: 0.5 }, { x: 500, y: 150 })
    expect(result.side).toBe('right')
    expect(result.point.x).toBe(300)
  })

  it('computes perimeter point from edge focus', () => {
    // Focus on the right edge midpoint, aiming right
    const result = focusToPerimeter(box, { fx: 1, fy: 0.5 }, { x: 500, y: 150 })
    expect(result.side).toBe('right')
    expect(result.point.x).toBe(300)
  })
})

// ============================================================================
// anchorToFocus
// ============================================================================

describe('anchorToFocus', () => {
  it('converts top anchor to focus on top edge', () => {
    const focus = anchorToFocus({ side: 'top', ratio: 0.3 })
    expect(focus.fx).toBe(0.3)
    expect(focus.fy).toBe(0)
  })

  it('converts bottom anchor to focus on bottom edge', () => {
    const focus = anchorToFocus({ side: 'bottom', ratio: 0.7 })
    expect(focus.fx).toBe(0.7)
    expect(focus.fy).toBe(1)
  })

  it('converts left anchor to focus on left edge', () => {
    const focus = anchorToFocus({ side: 'left', ratio: 0.5 })
    expect(focus.fx).toBe(0)
    expect(focus.fy).toBe(0.5)
  })

  it('converts right anchor to focus on right edge', () => {
    const focus = anchorToFocus({ side: 'right', ratio: 0.5 })
    expect(focus.fx).toBe(1)
    expect(focus.fy).toBe(0.5)
  })

  it('converts midpoint anchor correctly', () => {
    const focus = anchorToFocus({ side: 'top', ratio: 0.5 })
    expect(focus.fx).toBe(0.5)
    expect(focus.fy).toBe(0)
  })
})

// ============================================================================
// computeGeometricFocus
// ============================================================================

describe('computeGeometricFocus', () => {
  const box = makeBox(100, 100, 200, 100)

  it('produces focus facing left when source is to the left', () => {
    const focus = computeGeometricFocus(box, { x: 0, y: 150 })
    expect(focus.fx).toBe(0)  // left edge
    expect(focus.fy).toBe(0.5)
  })

  it('produces focus facing right when source is to the right', () => {
    const focus = computeGeometricFocus(box, { x: 500, y: 150 })
    expect(focus.fx).toBe(1)  // right edge
    expect(focus.fy).toBe(0.5)
  })

  it('produces focus facing top when source is above', () => {
    const focus = computeGeometricFocus(box, { x: 200, y: 0 })
    expect(focus.fx).toBe(0.5)
    expect(focus.fy).toBe(0)  // top edge
  })

  it('produces focus facing bottom when source is below', () => {
    const focus = computeGeometricFocus(box, { x: 200, y: 400 })
    expect(focus.fx).toBe(0.5)
    expect(focus.fy).toBe(1)  // bottom edge
  })

  it('wide box prefers horizontal sides', () => {
    const wideBox = makeBox(100, 100, 400, 100)
    const focus = computeGeometricFocus(wideBox, { x: 0, y: 150 })
    expect(focus.fx).toBe(0)  // left edge
  })

  it('tall box prefers vertical sides', () => {
    const tallBox = makeBox(100, 100, 100, 400)
    const focus = computeGeometricFocus(tallBox, { x: 150, y: 0 })
    expect(focus.fy).toBe(0)  // top edge
  })
})

// ============================================================================
// computeBindingAnchor (still used by edge hover)
// ============================================================================

describe('computeBindingAnchor', () => {
  const box = makeBox(100, 100, 200, 100)

  it('snaps to midpoint when cursor is near side center', () => {
    const anchor = computeBindingAnchor(box, { x: 200, y: 95 })
    expect(anchor.side).toBe('top')
    expect(anchor.ratio).toBe(0.5)
  })

  it('picks the nearest side', () => {
    const anchor = computeBindingAnchor(box, { x: 305, y: 150 })
    expect(anchor.side).toBe('right')
  })

  it('clamps ratio to avoid corners', () => {
    const anchor = computeBindingAnchor(box, { x: 110, y: 98 })
    expect(anchor.ratio).toBeGreaterThanOrEqual(0.1)
    expect(anchor.ratio).toBeLessThanOrEqual(0.9)
  })
})
