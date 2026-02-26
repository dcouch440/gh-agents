import { describe, expect, it, vi } from 'vitest'
import { fillOutlinePath } from './outlineToPath'

const createMockCtx = () => {
  const calls = {
    beginPath: 0,
    moveTo: 0,
    quadraticCurveTo: 0,
    lineTo: 0,
    closePath: 0,
    fill: 0,
  }
  const ctx = {
    beginPath: vi.fn(() => { calls.beginPath++ }),
    moveTo: vi.fn(() => { calls.moveTo++ }),
    quadraticCurveTo: vi.fn(() => { calls.quadraticCurveTo++ }),
    lineTo: vi.fn(() => { calls.lineTo++ }),
    closePath: vi.fn(() => { calls.closePath++ }),
    fill: vi.fn(() => { calls.fill++ }),
  } as unknown as CanvasRenderingContext2D
  return { ctx, calls }
}

describe('fillOutlinePath', () => {
  it('does nothing for fewer than 3 points', () => {
    const { ctx, calls } = createMockCtx()
    fillOutlinePath(ctx, [{ x: 0, y: 0 }, { x: 1, y: 1 }])
    expect(calls.beginPath).toBe(0)
  })

  it('draws a closed path with quadratic curves', () => {
    const { ctx, calls } = createMockCtx()
    const outline = [
      { x: 0, y: 0 },
      { x: 10, y: 5 },
      { x: 20, y: 0 },
      { x: 10, y: -5 },
    ]

    fillOutlinePath(ctx, outline)

    expect(calls.beginPath).toBe(1)
    expect(calls.moveTo).toBe(1)
    expect(calls.quadraticCurveTo).toBeGreaterThan(0)
    expect(calls.closePath).toBe(1)
    expect(calls.fill).toBe(1)
  })
})
