import { describe, it, expect } from 'vitest'
import { buildStepTree } from './buildStepTree'
import type { TreeEntry, GutterCell } from './buildStepTree'
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

const NO_ROSTER: Record<string, RosterAgent[]> = {}

/** Extract step IDs from entries, skipping gaps */
const stepIds = (entries: TreeEntry[]): string[] =>
  entries.filter((e): e is Extract<TreeEntry, { kind: 'step' }> => e.kind === 'step').map((e) => e.step.id)

/** Extract gutters from entries (gaps become 'gap') */
const gutters = (entries: TreeEntry[]): (readonly GutterCell[] | 'gap')[] =>
  entries.map((e) => (e.kind === 'gap' ? 'gap' : e.gutter))

// ── Tests ───────────────────────────────────────────────────────────────────

describe('buildStepTree', () => {
  it('returns empty array for empty steps', () => {
    expect(buildStepTree([], [], NO_ROSTER)).toEqual([])
  })

  it('returns single node with corner gutter', () => {
    const steps = [makeStep('a', 'Root', 0)]
    const result = buildStepTree(steps, [], NO_ROSTER)

    expect(stepIds(result)).toEqual(['a'])
    expect(gutters(result)).toEqual([['corner']])
  })

  // Pattern 1: Sequential A → B → C
  it('sequential chain', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
    ]
    const edges = [makeEdge('e1', 'a', 'b'), makeEdge('e2', 'b', 'c')]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c'])
    expect(gutters(result)).toEqual([['branch'], ['branch'], ['corner']])
  })

  // Pattern 2: Fan-out A → {B, C, D} → E
  it('parallel fan-out with single merge', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
      makeStep('e', 'E', 4),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
      makeEdge('e3', 'a', 'd'),
      makeEdge('e4', 'b', 'e'),
      makeEdge('e5', 'c', 'e'),
      makeEdge('e6', 'd', 'e'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd', 'e'])
    expect(gutters(result)).toEqual([
      ['branch'],           // ├── A
      ['fork_start'],       // ├─┬─ B
      ['pipe', 'par_mid'],  // │ ├─ C
      ['pipe', 'par_end'],  // │ └─ D
      ['corner'],           // └── E
    ])
  })

  // Pattern 3: Two sequential forks
  // A → {B, C} → D → {E, F} → G
  it('multiple sequential forks', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
      makeStep('e', 'E', 4),
      makeStep('f', 'F', 5),
      makeStep('g', 'G', 6),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
      makeEdge('e3', 'b', 'd'),
      makeEdge('e4', 'c', 'd'),
      makeEdge('e5', 'd', 'e'),
      makeEdge('e6', 'd', 'f'),
      makeEdge('e7', 'e', 'g'),
      makeEdge('e8', 'f', 'g'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd', 'e', 'f', 'g'])
    expect(gutters(result)).toEqual([
      ['branch'],           // ├── A
      ['fork_start'],       // ├─┬─ B
      ['pipe', 'par_end'],  // │ └─ C
      ['branch'],           // ├── D
      ['fork_start'],       // ├─┬─ E
      ['pipe', 'par_end'],  // │ └─ F
      ['corner'],           // └── G
    ])
  })

  // Pattern 4: Nested forks
  // A → {B, C} → G, B → {D, E} → F → G
  it('nested forks', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
      makeStep('e', 'E', 4),
      makeStep('f', 'F', 5),
      makeStep('g', 'G', 6),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
      makeEdge('e3', 'b', 'd'),
      makeEdge('e4', 'b', 'e'),
      makeEdge('e5', 'd', 'f'),
      makeEdge('e6', 'e', 'f'),
      makeEdge('e7', 'f', 'g'),
      makeEdge('e8', 'c', 'g'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'd', 'e', 'f', 'c', 'g'])
    expect(gutters(result)).toEqual([
      ['branch'],                       // ├── A
      ['fork_start'],                   // ├─┬─ B
      ['pipe', 'pipe', 'fork_start'],   // │ │  ├─┬─ D
      ['pipe', 'pipe', 'pipe', 'par_end'], // │ │  │ └─ E
      ['pipe', 'pipe', 'corner'],       // │ │  └── F
      ['pipe', 'par_end'],              // │ └─ C
      ['corner'],                       // └── G
    ])
  })

  // Pattern 5: Multiple independent roots
  // A → B, C → D (two separate chains)
  it('multiple independent roots', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
    ]
    const edges = [makeEdge('e1', 'a', 'b'), makeEdge('e2', 'c', 'd')]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd'])
    expect(gutters(result)).toEqual([
      ['branch'],  // ├── A
      ['corner'],  // └── B
      'gap',
      ['branch'],  // ├── C
      ['corner'],  // └── D
    ])
  })

  // Pattern 6: Fan-in from independent roots
  // A → C, B → C
  it('fan-in from independent roots', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
    ]
    const edges = [makeEdge('e1', 'a', 'c'), makeEdge('e2', 'b', 'c')]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c'])
    expect(gutters(result)).toEqual([
      ['root_fork'],  // ┬─ A
      ['par_end'],     // └─ B
      ['corner'],      // └── C
    ])
  })

  // Pattern 6 variant: Fan-in with continuation
  // A → C, B → C, C → D
  it('fan-in from independent roots with continuation', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
    ]
    const edges = [
      makeEdge('e1', 'a', 'c'),
      makeEdge('e2', 'b', 'c'),
      makeEdge('e3', 'c', 'd'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd'])
    expect(gutters(result)).toEqual([
      ['root_fork'],  // ┬─ A
      ['par_end'],     // └─ B
      ['branch'],      // ├── C
      ['corner'],      // └── D
    ])
  })

  // Pattern 8: Wide fan-out
  // A → {B, C, D, E, F} → G
  it('wide fan-out', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
      makeStep('e', 'E', 4),
      makeStep('f', 'F', 5),
      makeStep('g', 'G', 6),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
      makeEdge('e3', 'a', 'd'),
      makeEdge('e4', 'a', 'e'),
      makeEdge('e5', 'a', 'f'),
      makeEdge('e6', 'b', 'g'),
      makeEdge('e7', 'c', 'g'),
      makeEdge('e8', 'd', 'g'),
      makeEdge('e9', 'e', 'g'),
      makeEdge('e10', 'f', 'g'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd', 'e', 'f', 'g'])
    expect(gutters(result)).toEqual([
      ['branch'],           // ├── A
      ['fork_start'],       // ├─┬─ B
      ['pipe', 'par_mid'],  // │ ├─ C
      ['pipe', 'par_mid'],  // │ ├─ D
      ['pipe', 'par_mid'],  // │ ├─ E
      ['pipe', 'par_end'],  // │ └─ F
      ['corner'],           // └── G
    ])
  })

  // Pattern 9: Diamond
  // A → {B, C} → D
  it('diamond', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
      makeEdge('e3', 'b', 'd'),
      makeEdge('e4', 'c', 'd'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd'])
    expect(gutters(result)).toEqual([
      ['branch'],           // ├── A
      ['fork_start'],       // ├─┬─ B
      ['pipe', 'par_end'],  // │ └─ C
      ['corner'],           // └── D
    ])
  })

  // Pattern 10: Sequential with parallel section in middle
  // A → {B, C, D} → E → F
  it('sequential with parallel middle', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
      makeStep('e', 'E', 4),
      makeStep('f', 'F', 5),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
      makeEdge('e3', 'a', 'd'),
      makeEdge('e4', 'b', 'e'),
      makeEdge('e5', 'c', 'e'),
      makeEdge('e6', 'd', 'e'),
      makeEdge('e7', 'e', 'f'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd', 'e', 'f'])
    expect(gutters(result)).toEqual([
      ['branch'],           // ├── A
      ['fork_start'],       // ├─┬─ B
      ['pipe', 'par_mid'],  // │ ├─ C
      ['pipe', 'par_end'],  // │ └─ D
      ['branch'],           // ├── E
      ['corner'],           // └── F
    ])
  })

  // Edge cases
  it('sorts roots by display_order', () => {
    const steps = [
      makeStep('b', 'Second', 2),
      makeStep('a', 'First', 1),
    ]
    const result = buildStepTree(steps, [], NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b'])
  })

  it('includes orphaned steps', () => {
    const steps = [
      makeStep('a', 'Root', 0),
      makeStep('orphan', 'Orphan', 5),
    ]
    const edges = [makeEdge('e1', 'missing', 'orphan')]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(result).toHaveLength(3) // a, gap, orphan
    const ids = stepIds(result)
    expect(ids).toContain('a')
    expect(ids).toContain('orphan')
  })

  it('handles edges referencing missing steps gracefully', () => {
    const steps = [makeStep('a', 'A', 0), makeStep('b', 'B', 1)]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'b', 'missing'), // missing target
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b'])
  })

  it('ignores rosterByStep parameter', () => {
    const steps = [makeStep('a', 'A', 0)]
    const roster: Record<string, RosterAgent[]> = {
      a: [
        {
          id: 'agent-1',
          name: 'Worker',
          role_description: 'does work',
          capabilities: [],
          execution_order: 1,
          created_at: '2025-01-01T00:00:00Z',
          child_step_id: null,
          depends_on: [],
        },
      ],
    }
    const result = buildStepTree(steps, [], roster)

    // Should only have the step, no agent entries
    expect(result).toHaveLength(1)
    expect(result[0]!.kind).toBe('step')
  })

  it('filters out context and input steps', () => {
    const steps = [
      makeStep('a', 'A', 0),
      { ...makeStep('ctx', 'Context', 1), execution_mode: 'context' },
      makeStep('b', 'B', 2),
      { ...makeStep('inp', 'Input', 3), execution_mode: 'input' },
      makeStep('c', 'C', 4),
    ]
    const edges = [
      makeEdge('e1', 'a', 'ctx'),
      makeEdge('e2', 'ctx', 'b'),
      makeEdge('e3', 'b', 'inp'),
      makeEdge('e4', 'inp', 'c'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    // Only visible steps appear — context and input are excluded
    expect(stepIds(result)).toEqual(['a', 'b', 'c'])
  })

  it('handles two single-node components with gap', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
    ]
    const result = buildStepTree(steps, [], NO_ROSTER)

    expect(gutters(result)).toEqual([
      ['corner'],
      'gap',
      ['corner'],
    ])
  })
})
