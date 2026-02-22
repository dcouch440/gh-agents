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
  sub_workflow_template_id: null,
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
  describe('computePlacements', () => {
    it('returns empty array when all steps have positions', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b', position_x: 700, position_y: 0 }),
      ]
      expect(computePlacements(steps, [])).toHaveLength(0)
    })

    it('returns empty array when no steps exist', () => {
      expect(computePlacements([], [])).toHaveLength(0)
    })

    it('places single orphan step at origin', () => {
      const steps = [makeStep({ id: 'a' })]
      const results = computePlacements(steps, [])

      expect(results).toHaveLength(1)
      expect(results[0]!.stepId).toBe('a')
      expect(results[0]!.position.x % PLACEMENT.GRID_SIZE).toBe(0)
      expect(results[0]!.position.y % PLACEMENT.GRID_SIZE).toBe(0)
    })

    it('places single step to the right of placed upstream', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b' }),
      ]
      const edges = [makeEdge('a', 'b')]
      const results = computePlacements(steps, edges)

      expect(results).toHaveLength(1)
      expect(results[0]!.stepId).toBe('b')
      // 560 + 96 = 656 → snapToGrid(656, 24) = 648
      expect(results[0]!.position.x).toBe(648)
      expect(results[0]!.position.y).toBe(0)
    })

    it('places chain of 3 unplaced steps left-to-right', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b' }),
        makeStep({ id: 'c' }),
      ]
      const edges = [makeEdge('a', 'b'), makeEdge('b', 'c')]
      const results = computePlacements(steps, edges)

      expect(results).toHaveLength(2)

      const bResult = results.find((r) => r.stepId === 'b')!
      const cResult = results.find((r) => r.stepId === 'c')!

      // b is to the right of a
      expect(bResult.position.x).toBeGreaterThan(0)

      // c is to the right of b
      expect(cResult.position.x).toBeGreaterThan(bResult.position.x)

      // All grid-aligned
      expect(bResult.position.x % PLACEMENT.GRID_SIZE).toBe(0)
      expect(cResult.position.x % PLACEMENT.GRID_SIZE).toBe(0)
    })

    it('places mixed pipeline and orphan without overlap', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b' }), // connected to a
        makeStep({ id: 'c' }), // disconnected
      ]
      const edges = [makeEdge('a', 'b')]
      const results = computePlacements(steps, edges)

      expect(results).toHaveLength(2)

      const bResult = results.find((r) => r.stepId === 'b')!
      const cResult = results.find((r) => r.stepId === 'c')!

      // They should not overlap (accounting for default dimensions)
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
      const results = computePlacements(steps, edges)

      expect(results).toHaveLength(3)

      // Should be in left-to-right order
      const positions = results.map((r) => r.position.x)
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

      const results = computePlacements(steps, edges)
      expect(results).toHaveLength(10)

      // Check no overlaps between any pair of results + the original placed node
      const allRects: Rect[] = [{ x: 0, y: 0, width: 560, height: 500 }]
      for (const r of results) {
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

      const results1 = computePlacements(steps, edges)
      const results2 = computePlacements(steps, edges)

      expect(results1).toEqual(results2)
    })

    it('all positions are grid-aligned', () => {
      const steps = [
        makeStep({ id: 'a' }),
        makeStep({ id: 'b' }),
        makeStep({ id: 'c' }),
      ]
      const edges = [makeEdge('a', 'b'), makeEdge('b', 'c')]
      const results = computePlacements(steps, edges)

      for (const r of results) {
        expect(r.position.x % PLACEMENT.GRID_SIZE).toBe(0)
        expect(r.position.y % PLACEMENT.GRID_SIZE).toBe(0)
      }
    })
  })
})
