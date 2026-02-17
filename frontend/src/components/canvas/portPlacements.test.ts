import { describe, it, expect } from 'vitest'
import { CanvasNodeKind } from './canvasKinds'
import { getPortConfig, getPortsOnSide, getPortPosition, PORT_CONFIGS } from './portPlacements'
import type { Rect } from '@/utils/geometry'

describe('portPlacements', () => {
  // ── PORT_CONFIGS completeness ───────────────────────────────────────

  describe('PORT_CONFIGS', () => {
    it('has an entry for every CanvasNodeKind value', () => {
      const kindValues = Object.values(CanvasNodeKind) as CanvasNodeKind[]
      for (const kind of kindValues) {
        expect(PORT_CONFIGS[kind]).toBeDefined()
        expect(PORT_CONFIGS[kind].kind).toBe(kind)
        expect(PORT_CONFIGS[kind].ports.length).toBeGreaterThan(0)
      }
    })
  })

  // ── getPortConfig ───────────────────────────────────────────────────

  describe('getPortConfig', () => {
    it('returns step ports (left target, right source)', () => {
      const config = getPortConfig(CanvasNodeKind.STEP)
      expect(config.ports).toHaveLength(2)
      expect(config.ports[0]).toEqual({
        side: 'left', role: 'control-in', handleType: 'target', handleId: null,
      })
      expect(config.ports[1]).toEqual({
        side: 'right', role: 'control-out', handleType: 'source', handleId: null,
      })
    })

    it('returns protocol ports on all sides', () => {
      const config = getPortConfig(CanvasNodeKind.PROTOCOL)
      expect(config.ports.length).toBeGreaterThanOrEqual(4)

      const sides = new Set(config.ports.map((p) => p.side))
      expect(sides.has('left')).toBe(true)
      expect(sides.has('right')).toBe(true)
      expect(sides.has('top')).toBe(true)
      expect(sides.has('bottom')).toBe(true)
    })

    it('returns agent ports (bottom-in, top-out, right-docs)', () => {
      const config = getPortConfig(CanvasNodeKind.AGENT)
      expect(config.ports).toHaveLength(3)

      const roles = config.ports.map((p) => p.role)
      expect(roles).toContain('agent-input')
      expect(roles).toContain('agent-output')
      expect(roles).toContain('agent-documents')
    })

    it('returns context node with single bottom source', () => {
      const config = getPortConfig(CanvasNodeKind.CONTEXT)
      expect(config.ports).toHaveLength(1)
      expect(config.ports[0]!.side).toBe('bottom')
      expect(config.ports[0]!.handleType).toBe('source')
    })

    it('returns notes node with single top target', () => {
      const config = getPortConfig(CanvasNodeKind.NOTES)
      expect(config.ports).toHaveLength(1)
      expect(config.ports[0]!.side).toBe('top')
      expect(config.ports[0]!.handleType).toBe('target')
    })

    it('returns document node with single target', () => {
      const config = getPortConfig(CanvasNodeKind.DOCUMENT)
      expect(config.ports).toHaveLength(1)
      expect(config.ports[0]!.handleType).toBe('target')
    })

    it('returns sub_workflow with left-right flow', () => {
      const config = getPortConfig(CanvasNodeKind.SUB_WORKFLOW)
      expect(config.ports).toHaveLength(2)
      expect(config.ports[0]!.side).toBe('left')
      expect(config.ports[1]!.side).toBe('right')
    })
  })

  // ── getPortsOnSide ──────────────────────────────────────────────────

  describe('getPortsOnSide', () => {
    it('returns ports on the requested side', () => {
      const topPorts = getPortsOnSide(CanvasNodeKind.PROTOCOL, 'top')
      expect(topPorts.length).toBeGreaterThanOrEqual(1)
      for (const port of topPorts) {
        expect(port.side).toBe('top')
      }
    })

    it('returns empty for a side with no ports', () => {
      const topPorts = getPortsOnSide(CanvasNodeKind.STEP, 'top')
      expect(topPorts).toHaveLength(0)
    })

    it('returns multiple ports when side has multiple', () => {
      const topPorts = getPortsOnSide(CanvasNodeKind.PROTOCOL, 'top')
      expect(topPorts.length).toBe(2) // agents + documents
    })
  })

  // ── getPortPosition ─────────────────────────────────────────────────

  describe('getPortPosition', () => {
    const rect: Rect = { x: 100, y: 200, width: 400, height: 300 }

    it('returns centered position for single port on a side', () => {
      const pos = getPortPosition(CanvasNodeKind.CONTEXT, rect, 'control-out')
      expect(pos).toEqual({ x: 300, y: 500 }) // bottom center
    })

    it('returns null for non-existent role', () => {
      const pos = getPortPosition(CanvasNodeKind.STEP, rect, 'agents')
      expect(pos).toBeNull()
    })

    it('spaces multiple ports evenly on the same side', () => {
      // Protocol has 2 ports on top: agents + documents
      const agentsPos = getPortPosition(CanvasNodeKind.PROTOCOL, rect, 'agents')
      const docsPos = getPortPosition(CanvasNodeKind.PROTOCOL, rect, 'documents')

      expect(agentsPos).not.toBeNull()
      expect(docsPos).not.toBeNull()

      // Both should be on the top edge (y = rect.y)
      expect(agentsPos!.y).toBe(200)
      expect(docsPos!.y).toBe(200)

      // Should be evenly spaced: 1/3 and 2/3 of width
      expect(agentsPos!.x).toBeCloseTo(100 + 400 / 3, 5)
      expect(docsPos!.x).toBeCloseTo(100 + (400 * 2) / 3, 5)
    })

    it('returns left side center for step control-in', () => {
      const pos = getPortPosition(CanvasNodeKind.STEP, rect, 'control-in')
      expect(pos).toEqual({ x: 100, y: 350 }) // left center
    })

    it('returns right side center for step control-out', () => {
      const pos = getPortPosition(CanvasNodeKind.STEP, rect, 'control-out')
      expect(pos).toEqual({ x: 500, y: 350 }) // right center
    })

    it('returns top center for notes-input', () => {
      const pos = getPortPosition(CanvasNodeKind.NOTES, rect, 'notes-input')
      expect(pos).toEqual({ x: 300, y: 200 }) // top center
    })
  })
})
