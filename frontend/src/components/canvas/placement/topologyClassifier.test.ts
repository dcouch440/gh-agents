import { describe, it, expect } from 'vitest'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import { classifyPlacements, resolveStepDimensions } from './topologyClassifier'

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

describe('topologyClassifier', () => {
  describe('classifyPlacements', () => {
    it('returns empty array when all steps are placed', () => {
      const steps = [makeStep({ id: 'a', position_x: 0, position_y: 0 })]
      expect(classifyPlacements(steps, [])).toHaveLength(0)
    })

    it('returns empty array when no steps exist', () => {
      expect(classifyPlacements([], [])).toHaveLength(0)
    })

    it('classifies single unplaced step with no edges as free_space', () => {
      const steps = [makeStep({ id: 'a' })]
      const intents = classifyPlacements(steps, [])

      expect(intents).toHaveLength(1)
      expect(intents[0]!.stepId).toBe('a')
      expect(intents[0]!.strategy).toBe('free_space')
      expect(intents[0]!.upstreamStepId).toBeNull()
    })

    it('classifies unplaced step with placed upstream as pipeline', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b' }),
      ]
      const edges = [makeEdge('a', 'b')]
      const intents = classifyPlacements(steps, edges)

      expect(intents).toHaveLength(1)
      expect(intents[0]!.stepId).toBe('b')
      expect(intents[0]!.strategy).toBe('pipeline')
      expect(intents[0]!.upstreamStepId).toBe('a')
    })

    it('produces topo order for chain: placed → unplaced → unplaced', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b' }),
        makeStep({ id: 'c' }),
      ]
      const edges = [makeEdge('a', 'b'), makeEdge('b', 'c')]
      const intents = classifyPlacements(steps, edges)

      expect(intents).toHaveLength(2)
      // b should come before c (topo order)
      expect(intents[0]!.stepId).toBe('b')
      expect(intents[0]!.strategy).toBe('pipeline')
      expect(intents[0]!.upstreamStepId).toBe('a')

      // c's upstream should be b (which is now effectively placed)
      expect(intents[1]!.stepId).toBe('c')
      expect(intents[1]!.strategy).toBe('pipeline')
      expect(intents[1]!.upstreamStepId).toBe('b')
    })

    it('classifies two disconnected unplaced steps as free_space', () => {
      const steps = [makeStep({ id: 'a' }), makeStep({ id: 'b' })]
      const intents = classifyPlacements(steps, [])

      expect(intents).toHaveLength(2)
      expect(intents[0]!.strategy).toBe('free_space')
      expect(intents[1]!.strategy).toBe('free_space')
    })

    it('handles mixed strategies: one pipeline, one free_space', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b' }), // connected to a → pipeline
        makeStep({ id: 'c' }), // no edges → free_space
      ]
      const edges = [makeEdge('a', 'b')]
      const intents = classifyPlacements(steps, edges)

      expect(intents).toHaveLength(2)
      const pipelineIntent = intents.find((i) => i.stepId === 'b')
      const freeIntent = intents.find((i) => i.stepId === 'c')

      expect(pipelineIntent!.strategy).toBe('pipeline')
      expect(freeIntent!.strategy).toBe('free_space')
    })

    it('populates downstreamStepIds from edge adjacency', () => {
      const steps = [
        makeStep({ id: 'a', position_x: 0, position_y: 0 }),
        makeStep({ id: 'b' }),
        makeStep({ id: 'c' }),
      ]
      const edges = [makeEdge('a', 'b'), makeEdge('b', 'c')]
      const intents = classifyPlacements(steps, edges)

      const bIntent = intents.find((i) => i.stepId === 'b')
      expect(bIntent!.downstreamStepIds).toContain('c')
    })

    it('handles circular dependencies without infinite loop', () => {
      const steps = [makeStep({ id: 'a' }), makeStep({ id: 'b' })]
      const edges = [makeEdge('a', 'b'), makeEdge('b', 'a')]
      const intents = classifyPlacements(steps, edges)

      // Both should be classified (no hang)
      expect(intents).toHaveLength(2)
    })

    it('resolves correct dimensions from execution_mode', () => {
      const steps = [makeStep({ id: 'a', execution_mode: 'context' })]
      const intents = classifyPlacements(steps, [])

      // context variant defaults to 560×500
      expect(intents[0]!.width).toBe(560)
      expect(intents[0]!.height).toBe(500)
    })

    it('uses step width/height override when present', () => {
      const steps = [makeStep({ id: 'a', width: 800, height: 600 })]
      const intents = classifyPlacements(steps, [])

      expect(intents[0]!.width).toBe(800)
      expect(intents[0]!.height).toBe(600)
    })
  })

  describe('resolveStepDimensions', () => {
    it('returns defaults for workforce variant', () => {
      const step = makeStep({ id: 'a', execution_mode: 'workforce' })
      const dims = resolveStepDimensions(step)
      expect(dims.width).toBe(560)
      expect(dims.height).toBe(500)
    })

    it('respects explicit width/height on step', () => {
      const step = makeStep({ id: 'a', width: 300, height: 200 })
      const dims = resolveStepDimensions(step)
      expect(dims.width).toBe(300)
      expect(dims.height).toBe(200)
    })
  })
})
