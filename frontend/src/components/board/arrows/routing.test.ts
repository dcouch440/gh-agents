import { describe, expect, it } from 'vitest'
import type { BoxElement, FocusPoint } from '../elements'
import { computeArrowPathPoints } from './routing'

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
// computeArrowPathPoints
// ============================================================================

describe('computeArrowPathPoints', () => {
  it('produces a valid path between two horizontally aligned boxes', () => {
    const source = makeBox(0, 0, 200, 100)
    const target = makeBox(400, 0, 200, 100)
    const sourceFocus: FocusPoint = { fx: 1, fy: 0.5 }   // right edge center
    const targetFocus: FocusPoint = { fx: 0, fy: 0.5 }   // left edge center

    const path = computeArrowPathPoints(source, sourceFocus, target, targetFocus)

    // Start should be near right edge of source (with binding gap)
    expect(path.start.x).toBeGreaterThan(200)
    expect(path.start.y).toBeCloseTo(50, 0)

    // End should be near left edge of target (with binding gap)
    expect(path.end.x).toBeLessThan(400)
    expect(path.end.y).toBeCloseTo(50, 0)

    // Control points should extend outward
    expect(path.cp1.x).toBeGreaterThan(path.start.x)
    expect(path.cp2.x).toBeLessThan(path.end.x)
  })

  it('produces a valid path between vertically aligned boxes', () => {
    const source = makeBox(0, 0, 200, 100)
    const target = makeBox(0, 300, 200, 100)
    const sourceFocus: FocusPoint = { fx: 0.5, fy: 1 }   // bottom center
    const targetFocus: FocusPoint = { fx: 0.5, fy: 0 }   // top center

    const path = computeArrowPathPoints(source, sourceFocus, target, targetFocus)

    // Start should be near bottom of source
    expect(path.start.y).toBeGreaterThan(100)

    // End should be near top of target
    expect(path.end.y).toBeLessThan(300)
  })

  it('dynamically computes exit side when boxes move', () => {
    const source = makeBox(0, 0, 200, 100)
    const sourceFocus: FocusPoint = { fx: 0.5, fy: 0.5 }  // center
    const targetFocus: FocusPoint = { fx: 0.5, fy: 0.5 }  // center

    // Target to the right — should exit source from right side
    const targetRight = makeBox(400, 0, 200, 100)
    const pathRight = computeArrowPathPoints(source, sourceFocus, targetRight, targetFocus)
    expect(pathRight.start.x).toBeGreaterThan(100)  // exits right side

    // Same source focus, but target now below — should exit bottom side
    const targetBelow = makeBox(0, 300, 200, 100)
    const pathBelow = computeArrowPathPoints(source, sourceFocus, targetBelow, targetFocus)
    expect(pathBelow.start.y).toBeGreaterThan(50)   // exits bottom side
  })

  it('start and end points are always outside the box bounds (binding gap)', () => {
    const source = makeBox(0, 0, 200, 100)
    const target = makeBox(300, 0, 200, 100)
    const sourceFocus: FocusPoint = { fx: 1, fy: 0.5 }
    const targetFocus: FocusPoint = { fx: 0, fy: 0.5 }

    const path = computeArrowPathPoints(source, sourceFocus, target, targetFocus)

    // Start should be outside source box (right edge is at x=200)
    expect(path.start.x).toBeGreaterThan(200)

    // End should be outside target box (left edge is at x=300)
    expect(path.end.x).toBeLessThan(300)
  })
})
