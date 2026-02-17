import { describe, it, expect } from 'vitest'
import { CanvasNodeKind } from './canvasKinds'
import { getNodeDimensions, nodeToRect, toResizeConstraints, NODE_DIMENSIONS } from './nodeDimensions'

describe('nodeDimensions', () => {
  // ── getNodeDimensions ───────────────────────────────────────────────

  describe('getNodeDimensions', () => {
    const allKinds: CanvasNodeKind[] = [
      CanvasNodeKind.STEP,
      CanvasNodeKind.PROTOCOL,
      CanvasNodeKind.AGENT,
      CanvasNodeKind.CONTEXT,
      CanvasNodeKind.INPUT,
      CanvasNodeKind.DOCUMENT,
      CanvasNodeKind.NOTES,
      CanvasNodeKind.SUB_WORKFLOW,
    ]

    it('returns valid dimensions for every CanvasNodeKind', () => {
      for (const kind of allKinds) {
        const dims = getNodeDimensions(kind)
        expect(dims.defaultWidth).toBeGreaterThan(0)
        expect(dims.defaultHeight).toBeGreaterThan(0)
        expect(dims.minWidth).toBeLessThanOrEqual(dims.defaultWidth)
        expect(dims.minHeight).toBeLessThanOrEqual(dims.defaultHeight)
        expect(dims.maxWidth).toBeGreaterThanOrEqual(dims.defaultWidth)
        expect(dims.maxHeight).toBeGreaterThanOrEqual(dims.defaultHeight)
      }
    })

    it('returns correct values for context node', () => {
      const dims = getNodeDimensions(CanvasNodeKind.CONTEXT)
      expect(dims.defaultWidth).toBe(420)
      expect(dims.defaultHeight).toBe(360)
      expect(dims.minWidth).toBe(360)
      expect(dims.minHeight).toBe(300)
      expect(dims.maxWidth).toBe(1800)
      expect(dims.maxHeight).toBe(1600)
    })

    it('returns correct values for notes node', () => {
      const dims = getNodeDimensions(CanvasNodeKind.NOTES)
      expect(dims.defaultWidth).toBe(360)
      expect(dims.defaultHeight).toBe(300)
      expect(dims.minWidth).toBe(300)
      expect(dims.minHeight).toBe(240)
      expect(dims.maxWidth).toBe(1200)
      expect(dims.maxHeight).toBe(1200)
    })

    it('returns fixed dimensions for sub_workflow node', () => {
      const dims = getNodeDimensions(CanvasNodeKind.SUB_WORKFLOW)
      expect(dims.defaultWidth).toBe(dims.minWidth)
      expect(dims.defaultWidth).toBe(dims.maxWidth)
      expect(dims.defaultHeight).toBe(dims.minHeight)
      expect(dims.defaultHeight).toBe(dims.maxHeight)
    })

    it('returns fixed dimensions for step node', () => {
      const dims = getNodeDimensions(CanvasNodeKind.STEP)
      expect(dims.defaultWidth).toBe(dims.minWidth)
      expect(dims.defaultWidth).toBe(dims.maxWidth)
    })
  })

  // ── nodeToRect ──────────────────────────────────────────────────────

  describe('nodeToRect', () => {
    it('uses actual dimensions when provided', () => {
      const rect = nodeToRect({
        position: { x: 100, y: 200 },
        width: 500,
        height: 400,
        data: { kind: CanvasNodeKind.PROTOCOL },
      })
      expect(rect).toEqual({ x: 100, y: 200, width: 500, height: 400 })
    })

    it('falls back to defaults when dimensions are null', () => {
      const rect = nodeToRect({
        position: { x: 100, y: 200 },
        width: null,
        height: null,
        data: { kind: CanvasNodeKind.CONTEXT },
      })
      expect(rect).toEqual({ x: 100, y: 200, width: 420, height: 360 })
    })

    it('falls back to defaults when dimensions are undefined', () => {
      const rect = nodeToRect({
        position: { x: 0, y: 0 },
        data: { kind: CanvasNodeKind.NOTES },
      })
      expect(rect).toEqual({ x: 0, y: 0, width: 360, height: 300 })
    })

    it('handles mixed null and explicit dimensions', () => {
      const rect = nodeToRect({
        position: { x: 50, y: 50 },
        width: 800,
        height: null,
        data: { kind: CanvasNodeKind.DOCUMENT },
      })
      expect(rect).toEqual({ x: 50, y: 50, width: 800, height: 360 })
    })
  })

  // ── toResizeConstraints ─────────────────────────────────────────────

  describe('toResizeConstraints', () => {
    it('returns correct constraints for context node', () => {
      const c = toResizeConstraints(CanvasNodeKind.CONTEXT)
      expect(c).toEqual({
        minWidth: 360,
        minHeight: 300,
        maxWidth: 1800,
        maxHeight: 1600,
      })
    })

    it('returns correct constraints for notes node', () => {
      const c = toResizeConstraints(CanvasNodeKind.NOTES)
      expect(c).toEqual({
        minWidth: 300,
        minHeight: 240,
        maxWidth: 1200,
        maxHeight: 1200,
      })
    })

    it('min equals max for fixed-size nodes', () => {
      const c = toResizeConstraints(CanvasNodeKind.SUB_WORKFLOW)
      expect(c.minWidth).toBe(c.maxWidth)
      expect(c.minHeight).toBe(c.maxHeight)
    })
  })

  // ── NODE_DIMENSIONS completeness ────────────────────────────────────

  describe('NODE_DIMENSIONS', () => {
    it('has an entry for every CanvasNodeKind value', () => {
      const kindValues = Object.values(CanvasNodeKind) as CanvasNodeKind[]
      for (const kind of kindValues) {
        expect(NODE_DIMENSIONS[kind]).toBeDefined()
      }
    })
  })
})
