import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { LOD, DetailLevel } from './constants'

// ── Mocks ────────────────────────────────────────────────────────────────────

let selectorCapture: ((state: { transform: [number, number, number] }) => unknown) | null = null

vi.mock('@xyflow/react', () => ({
  useStore: (selector: (state: { transform: [number, number, number] }) => unknown) => {
    selectorCapture = selector
    return selector({ transform: [0, 0, 1] })
  },
}))

// Re-import after mock is set up
const { useCanvasLOD } = await import('./useCanvasLOD')

// ── Tests ────────────────────────────────────────────────────────────────────

describe('useCanvasLOD', () => {
  beforeEach(() => {
    selectorCapture = null
  })

  it('returns FULL at default zoom (1.0)', () => {
    const { result } = renderHook(() => useCanvasLOD())
    expect(result.current).toBe(DetailLevel.FULL)
  })

  it('selector transitions to MINIMAL when zoom drops below THRESHOLD_DOWN', () => {
    renderHook(() => useCanvasLOD())
    expect(selectorCapture).not.toBeNull()

    const belowThreshold = LOD.THRESHOLD_DOWN - 0.01
    const level = selectorCapture!({ transform: [0, 0, belowThreshold] })
    expect(level).toBe(DetailLevel.MINIMAL)
  })

  it('selector stays MINIMAL until zoom exceeds THRESHOLD_UP (hysteresis)', () => {
    renderHook(() => useCanvasLOD())
    expect(selectorCapture).not.toBeNull()

    // First drop below threshold
    selectorCapture!({ transform: [0, 0, LOD.THRESHOLD_DOWN - 0.01] })

    // Zoom between thresholds — should stay MINIMAL
    const midZoom = (LOD.THRESHOLD_DOWN + LOD.THRESHOLD_UP) / 2
    const level = selectorCapture!({ transform: [0, 0, midZoom] })
    expect(level).toBe(DetailLevel.MINIMAL)
  })

  it('selector transitions back to FULL when zoom exceeds THRESHOLD_UP', () => {
    renderHook(() => useCanvasLOD())
    expect(selectorCapture).not.toBeNull()

    // Drop to MINIMAL
    selectorCapture!({ transform: [0, 0, LOD.THRESHOLD_DOWN - 0.01] })

    // Rise above THRESHOLD_UP
    const level = selectorCapture!({ transform: [0, 0, LOD.THRESHOLD_UP + 0.01] })
    expect(level).toBe(DetailLevel.FULL)
  })

  it('selector stays FULL when zoom is above THRESHOLD_DOWN', () => {
    renderHook(() => useCanvasLOD())
    expect(selectorCapture).not.toBeNull()

    const aboveThreshold = LOD.THRESHOLD_DOWN + 0.01
    const level = selectorCapture!({ transform: [0, 0, aboveThreshold] })
    expect(level).toBe(DetailLevel.FULL)
  })
})
