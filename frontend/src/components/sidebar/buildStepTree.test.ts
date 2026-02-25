import { describe, it, expect } from 'vitest'
import { buildStepTree } from './buildStepTree'
import type { TreeEntry } from './buildStepTree'
import type { WorkflowStep, WorkflowStepEdge, RosterAgent } from '@/types/workflow'

// ── Helpers ─────────────────────────────────────────────────────────────────

const makeStep = (id: string, name: string, order: number): WorkflowStep => ({
  id,
  workflow_id: 'wf-1',
  agent_id: 'agent-1',
  execution_mode: 'single',
  for_each_ref: null,
  prompt_template_id: null,
  prompt_template: '',
  output_schema_id: null,
  output_variable_name: null,
  interactive_agent_id: null,
  for_each_label_field: null,
  display_order: order,
  version: 1,
  reasoning_trace: false,
  verification_agent_ids: [],
  position_x: null,
  position_y: null,
  width: null,
  height: null,
  name,
  room_id: null,
  system_prompt_suffix: null,
  description: `${name} description`,
  sub_workflow_template_id: null,
  pinned: false,
  run_results_summary: '',
})

const makeEdge = (id: string, from: string, to: string): WorkflowStepEdge => ({
  id,
  from_step_id: from,
  to_step_id: to,
})

const makeAgent = (id: string, name: string, order: number): RosterAgent => ({
  id,
  name,
  role_description: `${name} role`,
  capabilities: [],
  execution_order: order,
  created_at: '2025-01-01T00:00:00Z',
  child_step_id: null,
  depends_on: [],
})

const stepId = (entry: TreeEntry): string => {
  if (entry.kind === 'step') return entry.step.id
  return entry.agent.id
}

const NO_ROSTER: Record<string, RosterAgent[]> = {}

// ── Tests ───────────────────────────────────────────────────────────────────

describe('buildStepTree', () => {
  it('returns empty array for empty steps', () => {
    expect(buildStepTree([], [], NO_ROSTER)).toEqual([])
  })

  it('returns single root with no edges', () => {
    const steps = [makeStep('a', 'Root', 0)]
    const result = buildStepTree(steps, [], NO_ROSTER)

    expect(result).toHaveLength(1)
    expect(result[0]!.kind).toBe('step')
    expect(stepId(result[0]!)).toBe('a')
    expect(result[0]!.depth).toBe(0)
    expect(result[0]!.isLast).toBe(true)
  })

  it('builds a linear chain', () => {
    const steps = [
      makeStep('a', 'First', 0),
      makeStep('b', 'Second', 1),
      makeStep('c', 'Third', 2),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'b', 'c'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)
    const stepEntries = result.filter((e) => e.kind === 'step')

    expect(stepEntries).toHaveLength(3)
    expect(stepId(stepEntries[0]!)).toBe('a')
    expect(stepEntries[0]!.depth).toBe(0)
    expect(stepId(stepEntries[1]!)).toBe('b')
    expect(stepEntries[1]!.depth).toBe(1)
    expect(stepId(stepEntries[2]!)).toBe('c')
    expect(stepEntries[2]!.depth).toBe(2)
  })

  it('builds a branching tree', () => {
    const steps = [
      makeStep('root', 'Root', 0),
      makeStep('left', 'Left', 1),
      makeStep('right', 'Right', 2),
    ]
    const edges = [
      makeEdge('e1', 'root', 'left'),
      makeEdge('e2', 'root', 'right'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)
    const stepEntries = result.filter((e) => e.kind === 'step')

    expect(stepEntries).toHaveLength(3)
    expect(stepId(stepEntries[1]!)).toBe('left')
    expect(stepEntries[1]!.isLast).toBe(false)
    expect(stepId(stepEntries[2]!)).toBe('right')
    expect(stepEntries[2]!.isLast).toBe(true)
  })

  it('handles merge points without duplication', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
    ]
    const edges = [
      makeEdge('e1', 'a', 'c'),
      makeEdge('e2', 'b', 'c'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)
    const cEntries = result.filter((e) => e.kind === 'step' && e.step.id === 'c')
    expect(cEntries).toHaveLength(1)
  })

  it('sorts roots by display_order', () => {
    const steps = [
      makeStep('b', 'Second', 2),
      makeStep('a', 'First', 1),
    ]
    const result = buildStepTree(steps, [], NO_ROSTER)

    expect(stepId(result[0]!)).toBe('a')
    expect(stepId(result[1]!)).toBe('b')
  })

  it('includes roster agents under their step', () => {
    const steps = [makeStep('s1', 'Research', 0)]
    const roster: Record<string, RosterAgent[]> = {
      s1: [
        makeAgent('a1', 'Web Searcher', 1),
        makeAgent('a2', 'Fact Checker', 2),
      ],
    }
    const result = buildStepTree(steps, [], roster)

    expect(result).toHaveLength(3)
    expect(result[0]!.kind).toBe('step')
    expect(result[1]!.kind).toBe('agent')
    expect(result[2]!.kind).toBe('agent')
    expect(stepId(result[1]!)).toBe('a1')
    expect(stepId(result[2]!)).toBe('a2')
    expect(result[1]!.depth).toBe(1) // indented under step
    expect(result[2]!.isLast).toBe(true)
  })

  it('filters out manager agents', () => {
    const steps = [makeStep('s1', 'Research', 0)]
    const roster: Record<string, RosterAgent[]> = {
      s1: [
        makeAgent('mgr', 'Manager', 0),
        makeAgent('a1', 'Worker', 1),
        makeAgent('des', 'Designer', 2),
      ],
    }
    const result = buildStepTree(steps, [], roster)
    const agentEntries = result.filter((e) => e.kind === 'agent')

    expect(agentEntries).toHaveLength(1)
    expect(stepId(agentEntries[0]!)).toBe('a1')
  })

  it('includes orphaned steps', () => {
    const steps = [
      makeStep('a', 'Root', 0),
      makeStep('orphan', 'Orphan', 5),
    ]
    const edges = [makeEdge('e1', 'missing', 'orphan')]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(result).toHaveLength(2)
    const orphan = result.find((e) => e.kind === 'step' && e.step.id === 'orphan')
    expect(orphan).toBeDefined()
  })
})
