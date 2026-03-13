import { describe, it, expect } from 'vitest'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import type { Rect } from '@/utils/geometry'
import { Geometry } from '@/utils/geometry'
import { computePlacements } from './placementEngine'
import { PLACEMENT } from './constants'

const makeStep = (overrides: Partial<WorkflowStep> & { id: string }): WorkflowStep => ({
  id: overrides.id,
  workflow_id: 'wf-test',
  agent_id: 'agent-test',
  execution_mode: 'workforce',
  for_each_ref: null,
  prompt_template_id: null,
  prompt_template: '',
  output_schema_id: null,
  output_variable_name: null,
  interactive_agent_id: null,
  for_each_label_field: null,
  display_order: 0,
  version: 1,
  reasoning_trace: false,
  verification_agent_ids: [],
  position_x: null,
  position_y: null,
  width: null,
  height: null,
  name: null,
  room_id: null,
  system_prompt_suffix: null,
  description: '',

  pinned: false,
  run_results_summary: '',
  ...overrides,
})

const makeEdge = (from: string, to: string): WorkflowStepEdge => ({
  id: `edge-${from}-${to}`,
  from_step_id: from,
  to_step_id: to,
})

describe('placementEngine', () => {
  describe('computePlacements — pipeline (Phase 1)', () => {
    it('returns empty output when all steps have positions', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b', position_x: 700, position_y: 0 }),
      ]
      const output = computePlacements(steps, [])
      expect(output.placements).toHaveLength(0)
      expect(output.shifts).toHaveLength(0)
    })

    it('returns empty output when no steps exist', () => {
      const output = computePlacements([], [])
      expect(output.placements).toHaveLength(0)
      expect(output.shifts).toHaveLength(0)
    })

    it('places single orphan step at origin', () => {
      const steps = [makeStep({ id: 'a' })]
      const output = computePlacements(steps, [])

      expect(output.placements).toHaveLength(1)
      expect(output.placements[0]!.stepId).toBe('a')
      expect(output.placements[0]!.position.x % PLACEMENT.GRID_SIZE).toBe(0)
      expect(output.placements[0]!.position.y % PLACEMENT.GRID_SIZE).toBe(0)
    })

    it('places single step to the right of placed upstream', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b' }),
      ]
      const edges = [makeEdge('a', 'b')]
      const output = computePlacements(steps, edges)

      expect(output.placements).toHaveLength(1)
      expect(output.placements[0]!.stepId).toBe('b')
      // 560 + 96 = 656 → snapToGrid(656, 24) = 648
      expect(output.placements[0]!.position.x).toBe(648)
      expect(output.placements[0]!.position.y).toBe(0)
    })

    it('places chain of 3 unplaced steps left-to-right', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b' }),
        makeStep({ id: 'c' }),
      ]
      const edges = [makeEdge('a', 'b'), makeEdge('b', 'c')]
      const output = computePlacements(steps, edges)

      expect(output.placements).toHaveLength(2)

      const bResult = output.placements.find((r) => r.stepId === 'b')!
      const cResult = output.placements.find((r) => r.stepId === 'c')!

      expect(bResult.position.x).toBeGreaterThan(0)
      expect(cResult.position.x).toBeGreaterThan(bResult.position.x)
      expect(bResult.position.x % PLACEMENT.GRID_SIZE).toBe(0)
      expect(cResult.position.x % PLACEMENT.GRID_SIZE).toBe(0)
    })

    it('places mixed pipeline and orphan without overlap', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b' }),
        makeStep({ id: 'c' }),
      ]
      const edges = [makeEdge('a', 'b')]
      const output = computePlacements(steps, edges)

      expect(output.placements).toHaveLength(2)

      const bResult = output.placements.find((r) => r.stepId === 'b')!
      const cResult = output.placements.find((r) => r.stepId === 'c')!

      const bRect: Rect = { x: bResult.position.x, y: bResult.position.y, width: 560, height: 500 }
      const cRect: Rect = { x: cResult.position.x, y: cResult.position.y, width: 560, height: 500 }
      expect(Geometry.rectsOverlap(bRect, cRect)).toBe(false)
    })

    it('handles empty canvas with all unplaced', () => {
      const steps = [
        makeStep({ id: 'a' }),
        makeStep({ id: 'b' }),
        makeStep({ id: 'c' }),
      ]
      const edges = [makeEdge('a', 'b'), makeEdge('b', 'c')]
      const output = computePlacements(steps, edges)

      expect(output.placements).toHaveLength(3)

      const positions = output.placements.map((r) => r.position.x)
      for (let i = 1; i < positions.length; i++) {
        expect(positions[i]!).toBeGreaterThan(positions[i - 1]!)
      }
    })

    it('places 10-node chain without any overlaps', () => {
      const steps: WorkflowStep[] = [
        makeStep({ id: 's0', position_x: 0, position_y: 0 }),
      ]
      const edges: WorkflowStepEdge[] = []

      for (let i = 1; i <= 10; i++) {
        steps.push(makeStep({ id: `s${i}` }))
        edges.push(makeEdge(`s${i - 1}`, `s${i}`))
      }

      const output = computePlacements(steps, edges)
      expect(output.placements).toHaveLength(10)

      const allRects: Rect[] = [{ x: 0, y: 0, width: 560, height: 500 }]
      for (const r of output.placements) {
        allRects.push({ x: r.position.x, y: r.position.y, width: 560, height: 500 })
      }

      for (let i = 0; i < allRects.length; i++) {
        for (let j = i + 1; j < allRects.length; j++) {
          expect(Geometry.rectsOverlap(allRects[i]!, allRects[j]!)).toBe(false)
        }
      }
    })

    it('is deterministic — same inputs produce same outputs', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b' }),
        makeStep({ id: 'c' }),
      ]
      const edges = [makeEdge('a', 'b'), makeEdge('b', 'c')]

      const output1 = computePlacements(steps, edges)
      const output2 = computePlacements(steps, edges)

      expect(output1).toEqual(output2)
    })

    it('all positions are grid-aligned', () => {
      const steps = [
        makeStep({ id: 'a' }),
        makeStep({ id: 'b' }),
        makeStep({ id: 'c' }),
      ]
      const edges = [makeEdge('a', 'b'), makeEdge('b', 'c')]
      const output = computePlacements(steps, edges)

      for (const r of output.placements) {
        expect(r.position.x % PLACEMENT.GRID_SIZE).toBe(0)
        expect(r.position.y % PLACEMENT.GRID_SIZE).toBe(0)
      }
    })

    it('returns empty shifts for pipeline-only placements', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b' }),
      ]
      const edges = [makeEdge('a', 'b')]
      const output = computePlacements(steps, edges)

      expect(output.shifts).toHaveLength(0)
    })
  })

  describe('computePlacements — fan_out topology', () => {
    it('places fan-out children in vertical stack', () => {
      const steps = [
        makeStep({ id: 'source', position_x: 0, position_y: 0 }),
        makeStep({ id: 'a' }),
        makeStep({ id: 'b' }),
        makeStep({ id: 'c' }),
      ]
      const edges = [
        makeEdge('source', 'a'),
        makeEdge('source', 'b'),
        makeEdge('source', 'c'),
      ]
      const output = computePlacements(steps, edges)

      expect(output.placements).toHaveLength(3)

      // All at same X (to the right of source)
      const xs = output.placements.map((r) => r.position.x)
      expect(new Set(xs).size).toBe(1)
      expect(xs[0]!).toBeGreaterThan(0)

      // Vertically ordered
      const ys = output.placements.map((r) => r.position.y)
      for (let i = 1; i < ys.length; i++) {
        expect(ys[i]!).toBeGreaterThan(ys[i - 1]!)
      }
    })

    it('places fan-out with convergence target to the right of stack', () => {
      const steps = [
        makeStep({ id: 'source', position_x: 0, position_y: 0 }),
        makeStep({ id: 'a' }),
        makeStep({ id: 'b' }),
        makeStep({ id: 'target' }),
      ]
      const edges = [
        makeEdge('source', 'a'),
        makeEdge('source', 'b'),
        makeEdge('a', 'target'),
        makeEdge('b', 'target'),
      ]
      const output = computePlacements(steps, edges)

      expect(output.placements).toHaveLength(3) // a, b, target

      const targetResult = output.placements.find((r) => r.stepId === 'target')!
      const siblingResults = output.placements.filter((r) => r.stepId !== 'target')

      // Target should be further right than siblings
      for (const sib of siblingResults) {
        expect(targetResult.position.x).toBeGreaterThan(sib.position.x)
      }
    })

    it('no overlaps in complete fan-out/fan-in pattern', () => {
      const steps = [
        makeStep({ id: 'source', position_x: 0, position_y: 0 }),
        makeStep({ id: 'a' }),
        makeStep({ id: 'b' }),
        makeStep({ id: 'c' }),
        makeStep({ id: 'target' }),
      ]
      const edges = [
        makeEdge('source', 'a'),
        makeEdge('source', 'b'),
        makeEdge('source', 'c'),
        makeEdge('a', 'target'),
        makeEdge('b', 'target'),
        makeEdge('c', 'target'),
      ]
      const output = computePlacements(steps, edges)

      const allRects: Rect[] = [{ x: 0, y: 0, width: 560, height: 500 }]
      for (const r of output.placements) {
        allRects.push({ x: r.position.x, y: r.position.y, width: 560, height: 500 })
      }

      for (let i = 0; i < allRects.length; i++) {
        for (let j = i + 1; j < allRects.length; j++) {
          expect(Geometry.rectsOverlap(allRects[i]!, allRects[j]!)).toBe(false)
        }
      }
    })
  })

  describe('computePlacements — splice topology', () => {
    it('places splice node between two placed nodes with sufficient gap', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'new' }),
        makeStep({ id: 'b', position_x: 1500, position_y: 0 }),
      ]
      const edges = [makeEdge('a', 'new'), makeEdge('new', 'b')]
      const output = computePlacements(steps, edges)

      expect(output.placements).toHaveLength(1)
      expect(output.placements[0]!.stepId).toBe('new')
      // Should be between a and b
      expect(output.placements[0]!.position.x).toBeGreaterThan(0)
      expect(output.placements[0]!.position.x).toBeLessThan(1500)
      expect(output.shifts).toHaveLength(0)
    })

    it('shifts downstream when gap insufficient and node is shiftable', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'new' }),
        makeStep({ id: 'b', position_x: 660, position_y: 0 }),
      ]
      const edges = [makeEdge('a', 'new'), makeEdge('new', 'b')]
      const shiftableIds = new Set(['b'])
      const output = computePlacements(steps, edges, shiftableIds)

      expect(output.placements).toHaveLength(1)
      expect(output.shifts).toHaveLength(1)
      expect(output.shifts[0]!.stepId).toBe('b')
      expect(output.shifts[0]!.dx).toBeGreaterThan(0)
    })

    it('does not shift non-shiftable downstream nodes', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'new' }),
        makeStep({ id: 'b', position_x: 660, position_y: 0 }),
      ]
      const edges = [makeEdge('a', 'new'), makeEdge('new', 'b')]
      // Empty shiftableIds — b is NOT shiftable
      const output = computePlacements(steps, edges)

      expect(output.placements).toHaveLength(1)
      expect(output.shifts).toHaveLength(0)
      // b should NOT have moved (not in shifts)
    })

    it('all positions grid-aligned after splice', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'new' }),
        makeStep({ id: 'b', position_x: 1500, position_y: 0 }),
      ]
      const edges = [makeEdge('a', 'new'), makeEdge('new', 'b')]
      const output = computePlacements(steps, edges)

      for (const r of output.placements) {
        expect(r.position.x % PLACEMENT.GRID_SIZE).toBe(0)
        expect(r.position.y % PLACEMENT.GRID_SIZE).toBe(0)
      }
    })
  })

  describe('computePlacements — mixed topologies', () => {
    it('handles pipeline + fan_out in same batch', () => {
      const steps = [
        makeStep({ id: 'root', position_x: 0, position_y: 0 }),
        makeStep({ id: 'pipe' }),   // pipeline from root
        makeStep({ id: 'fan-source', position_x: 0, position_y: 600 }),
        makeStep({ id: 'fa' }),     // fan-out from fan-source
        makeStep({ id: 'fb' }),     // fan-out from fan-source
      ]
      const edges = [
        makeEdge('root', 'pipe'),
        makeEdge('fan-source', 'fa'),
        makeEdge('fan-source', 'fb'),
      ]
      const output = computePlacements(steps, edges)

      expect(output.placements).toHaveLength(3)
      const pipeResult = output.placements.find((r) => r.stepId === 'pipe')!
      const faResult = output.placements.find((r) => r.stepId === 'fa')!
      const fbResult = output.placements.find((r) => r.stepId === 'fb')!

      // Pipeline should be right of root
      expect(pipeResult.position.x).toBeGreaterThan(0)

      // Fan-out siblings should share same X
      expect(faResult.position.x).toBe(fbResult.position.x)
    })
  })
})
