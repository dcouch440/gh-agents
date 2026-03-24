import { describe, it, expect } from 'vitest'
import { buildStepTree } from './buildStepTree'
import type { TreeEntry, AgentEntry, GutterCell } from './buildStepTree'
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

  pinned: false,
  run_results_summary: '',
  designer_handoff: '',
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
      ['branch'],                // ├── A
      ['pipe', 'fork_start'],    // │ ┌─ B
      ['pipe', 'par_mid'],       // │ ├─ C
      ['pipe', 'par_end'],       // │ └─ D
      ['corner'],                // └── E
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
      ['branch'],                // ├── A
      ['pipe', 'fork_start'],    // │ ┌─ B
      ['pipe', 'par_end'],       // │ └─ C
      ['branch'],                // ├── D
      ['pipe', 'fork_start'],    // │ ┌─ E
      ['pipe', 'par_end'],       // │ └─ F
      ['corner'],                // └── G
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
      ['branch'],                                          // ├── A
      ['pipe', 'fork_start'],                              // │ ┌─ B
      ['pipe', 'pipe', 'pipe', 'pipe', 'fork_start'],     // │ │  │ │ ┌─ D
      ['pipe', 'pipe', 'pipe', 'pipe', 'par_end'],        // │ │  │ │ └─ E
      ['pipe', 'pipe', 'pipe', 'corner'],                  // │ │  └── F
      ['pipe', 'par_end'],                                 // │ └─ C
      ['corner'],                                          // └── G
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
      ['pipe', 'fork_start'],  // │ ┌─ A (indented input)
      ['pipe', 'par_end'],     // │ └─ B (indented input)
      ['corner'],              // └── C  (base level convergence)
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
      ['pipe', 'fork_start'],  // │ ┌─ A (indented input)
      ['pipe', 'par_end'],     // │ └─ B (indented input)
      ['branch'],              // ├── C  (base level convergence)
      ['corner'],              // └── D
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
      ['branch'],                // ├── A
      ['pipe', 'fork_start'],    // │ ┌─ B
      ['pipe', 'par_mid'],       // │ ├─ C
      ['pipe', 'par_mid'],       // │ ├─ D
      ['pipe', 'par_mid'],       // │ ├─ E
      ['pipe', 'par_end'],       // │ └─ F
      ['corner'],                // └── G
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
      ['branch'],                // ├── A
      ['pipe', 'fork_start'],    // │ ┌─ B
      ['pipe', 'par_end'],       // │ └─ C
      ['corner'],                // └── D
    ])
  })

  // Pattern 9b: Fork with direct edge to merge point (skip edge)
  // A → B, A → C, A → D, B → D, C → D, D → E
  // A forks to {B, C} and also has a direct edge to D (the merge point)
  it('fork with direct skip edge to merge', () => {
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
      makeEdge('e3', 'a', 'd'),  // direct skip to merge
      makeEdge('e4', 'b', 'd'),
      makeEdge('e5', 'c', 'd'),
      makeEdge('e6', 'd', 'e'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    // D is the merge point — it should appear AFTER the fork, not inside it
    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd', 'e'])
    expect(gutters(result)).toEqual([
      ['branch'],                // ├── A
      ['pipe', 'fork_start'],   // │ ┌─ B
      ['pipe', 'par_end'],      // │ └─ C
      ['branch'],                // ├── D (merge point, after fork)
      ['corner'],                // └── E
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
      ['branch'],                // ├── A
      ['pipe', 'fork_start'],    // │ ┌─ B
      ['pipe', 'par_mid'],       // │ ├─ C
      ['pipe', 'par_end'],       // │ └─ D
      ['branch'],                // ├── E
      ['corner'],                // └── F
    ])
  })

  // Fan-out without merge: A → {B, C}
  it('fan-out without merge', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c'])
    expect(gutters(result)).toEqual([
      ['branch'],                // ├── A
      ['pipe', 'fork_start'],    // │ ┌─ B
      ['pipe', 'par_end'],       // │ └─ C
    ])
  })

  // Wide fan-out without merge: A → {B, C, D}
  it('wide fan-out without merge', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
      makeEdge('e3', 'a', 'd'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd'])
    expect(gutters(result)).toEqual([
      ['branch'],                // ├── A
      ['pipe', 'fork_start'],    // │ ┌─ B
      ['pipe', 'par_mid'],       // │ ├─ C
      ['pipe', 'par_end'],       // │ └─ D
    ])
  })

  // Sequential prefix then fan-out without merge: X → A → {B, C}
  it('sequential then fan-out without merge', () => {
    const steps = [
      makeStep('x', 'X', 0),
      makeStep('a', 'A', 1),
      makeStep('b', 'B', 2),
      makeStep('c', 'C', 3),
    ]
    const edges = [
      makeEdge('e0', 'x', 'a'),
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['x', 'a', 'b', 'c'])
    expect(gutters(result)).toEqual([
      ['branch'],                // ├── X
      ['branch'],                // ├── A
      ['pipe', 'fork_start'],    // │ ┌─ B
      ['pipe', 'par_end'],       // │ └─ C
    ])
  })

  // Fan-out without merge, one branch has children: A → {B, C}, C → D
  it('fan-out without merge, branch with children', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
      makeEdge('e3', 'c', 'd'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd'])
    expect(gutters(result)).toEqual([
      ['branch'],                    // ├── A
      ['pipe', 'fork_start'],        // │ ┌─ B
      ['pipe', 'par_end'],           // │ └─ C
      ['pipe', 'pipe', 'corner'],    // │    └── D
    ])
  })

  // Nested fan-out without merge: A → {B, C}, B → {D, E}
  it('nested fan-out without merge', () => {
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
      makeEdge('e3', 'b', 'd'),
      makeEdge('e4', 'b', 'e'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'd', 'e', 'c'])
    expect(gutters(result)).toEqual([
      ['branch'],                                          // ├── A
      ['pipe', 'fork_start'],                              // │ ┌─ B
      ['pipe', 'pipe', 'pipe', 'pipe', 'fork_start'],     // │ │  │ │ ┌─ D
      ['pipe', 'pipe', 'pipe', 'pipe', 'par_end'],        // │ │  │ │ └─ E
      ['pipe', 'par_end'],                                 // │ └─ C
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

  it('ignores roster for non-workforce steps', () => {
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

    // Single-mode step should not produce agent entries even with roster
    expect(result).toHaveLength(1)
    expect(result[0]!.kind).toBe('step')
  })

  it('filters out context and input steps', () => {
    const steps = [
      makeStep('a', 'A', 0),
      { ...makeStep('ctx', 'Context', 1), execution_mode: 'context' as const },
      makeStep('b', 'B', 2),
      { ...makeStep('inp', 'Input', 3), execution_mode: 'input' as const },
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

  // ── Agent entry tests ──────────────────────────────────────────────────

  const makeWorkforceStep = (id: string, name: string, order: number): WorkflowStep => ({
    ...makeStep(id, name, order),
    execution_mode: 'workforce',
  })

  const makeRosterAgent = (name: string, executionOrder: number): RosterAgent => ({
    id: `roster-${name}`,
    name,
    role_description: `${name} role`,
    capabilities: [],
    execution_order: executionOrder,
    created_at: '2025-01-01T00:00:00Z',
    child_step_id: null,
    depends_on: [],
  })

  it('emits agent entries after workforce step', () => {
    const steps = [makeWorkforceStep('wf', 'Research Team', 0)]
    const roster: Record<string, RosterAgent[]> = {
      wf: [
        makeRosterAgent('Scanner', 0),
        makeRosterAgent('Writer', 1),
        makeRosterAgent('Reviewer', 2),
      ],
    }
    const result = buildStepTree(steps, [], roster)

    expect(result).toHaveLength(4) // 1 step + 3 agents
    expect(result[0]!.kind).toBe('step')
    expect(result[1]!.kind).toBe('agent')
    expect(result[2]!.kind).toBe('agent')
    expect(result[3]!.kind).toBe('agent')

    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')
    expect(agents.map((a) => a.agentName)).toEqual(['Scanner', 'Writer', 'Reviewer'])
    expect(agents.map((a) => a.stepId)).toEqual(['wf', 'wf', 'wf'])
  })

  it('agent gutter uses continuation + branch/corner', () => {
    const steps = [
      makeWorkforceStep('wf', 'Team', 0),
      makeStep('b', 'Next', 1),
    ]
    const edges = [makeEdge('e1', 'wf', 'b')]
    const roster: Record<string, RosterAgent[]> = {
      wf: [
        makeRosterAgent('Alpha', 0),
        makeRosterAgent('Beta', 1),
      ],
    }
    const result = buildStepTree(steps, edges, roster)

    // step wf gutter = ['branch'], continuation = ['pipe']
    // Alpha = ['pipe', 'branch'], Beta = ['pipe', 'corner']
    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')
    expect(agents).toHaveLength(2)
    expect(agents[0]!.gutter).toEqual(['pipe', 'branch'])
    expect(agents[1]!.gutter).toEqual(['pipe', 'corner'])
  })

  it('sorts agent entries by execution_order', () => {
    const steps = [makeWorkforceStep('wf', 'Team', 0)]
    const roster: Record<string, RosterAgent[]> = {
      wf: [
        makeRosterAgent('Zulu', 2),
        makeRosterAgent('Alpha', 0),
        makeRosterAgent('Mike', 1),
      ],
    }
    const result = buildStepTree(steps, [], roster)

    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')
    expect(agents.map((a) => a.agentName)).toEqual(['Alpha', 'Mike', 'Zulu'])
  })

  it('emits no agent entries for empty roster', () => {
    const steps = [makeWorkforceStep('wf', 'Team', 0)]
    const result = buildStepTree(steps, [], NO_ROSTER)

    expect(result).toHaveLength(1)
    expect(result[0]!.kind).toBe('step')
  })

  it('agents nest correctly inside parallel fork', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeWorkforceStep('b', 'B Team', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
      makeEdge('e3', 'b', 'd'),
      makeEdge('e4', 'c', 'd'),
    ]
    const roster: Record<string, RosterAgent[]> = {
      b: [makeRosterAgent('Worker', 0)],
    }
    const result = buildStepTree(steps, edges, roster)

    // A, B(fork_start), Worker(agent), C(par_end), D
    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')
    expect(agents).toHaveLength(1)
    expect(agents[0]!.agentName).toBe('Worker')

    // B's gutter is ['pipe', 'fork_start'], continuation = ['pipe', 'pipe']
    // Worker should be ['pipe', 'pipe', 'corner'] (single agent = last)
    expect(agents[0]!.gutter).toEqual(['pipe', 'pipe', 'corner'])
  })

  // ── Agent topology tests ────────────────────────────────────────────────

  const makeDepAgent = (name: string, order: number, dependsOn: string[]): RosterAgent => ({
    ...makeRosterAgent(name, order),
    depends_on: dependsOn,
  })

  // Note: solo workforce step gets gutter ['corner'], continuation = ['blank'].
  // Agent gutters are prefixed with this continuation.

  it('agent topology: sequential chain A → B → C', () => {
    const steps = [makeWorkforceStep('wf', 'Team', 0)]
    const roster: Record<string, RosterAgent[]> = {
      wf: [
        makeDepAgent('A', 0, []),
        makeDepAgent('B', 1, ['roster-A']),
        makeDepAgent('C', 2, ['roster-B']),
      ],
    }
    const result = buildStepTree(steps, [], roster)
    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')

    expect(agents.map((a) => a.agentName)).toEqual(['A', 'B', 'C'])
    expect(agents.map((a) => a.gutter)).toEqual([
      ['blank', 'branch'],   //   ├── A
      ['blank', 'branch'],   //   ├── B
      ['blank', 'corner'],   //   └── C
    ])
  })

  it('agent topology: parallel fork A → {B, C} → D', () => {
    const steps = [makeWorkforceStep('wf', 'Team', 0)]
    const roster: Record<string, RosterAgent[]> = {
      wf: [
        makeDepAgent('A', 0, []),
        makeDepAgent('B', 1, ['roster-A']),
        makeDepAgent('C', 2, ['roster-A']),
        makeDepAgent('D', 3, ['roster-B', 'roster-C']),
      ],
    }
    const result = buildStepTree(steps, [], roster)
    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')

    expect(agents.map((a) => a.agentName)).toEqual(['A', 'B', 'C', 'D'])
    expect(agents.map((a) => a.gutter)).toEqual([
      ['blank', 'branch'],                // ├── A (fork point)
      ['blank', 'pipe', 'fork_start'],   // │ ┌─ B
      ['blank', 'pipe', 'par_end'],      // │ └─ C
      ['blank', 'corner'],                // └── D (merge)
    ])
  })

  it('agent topology: all parallel roots (no depends_on)', () => {
    const steps = [makeWorkforceStep('wf', 'Team', 0)]
    const roster: Record<string, RosterAgent[]> = {
      wf: [
        makeDepAgent('X', 2, []),
        makeDepAgent('Y', 0, []),
        makeDepAgent('Z', 1, []),
      ],
    }
    const result = buildStepTree(steps, [], roster)
    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')

    // No edges → flat list sorted by execution_order
    expect(agents.map((a) => a.agentName)).toEqual(['Y', 'Z', 'X'])
    expect(agents.map((a) => a.gutter)).toEqual([
      ['blank', 'branch'],
      ['blank', 'branch'],
      ['blank', 'corner'],
    ])
  })

  it('agent topology: fan-out without merge A → {B, C}', () => {
    const steps = [makeWorkforceStep('wf', 'Team', 0)]
    const roster: Record<string, RosterAgent[]> = {
      wf: [
        makeDepAgent('A', 0, []),
        makeDepAgent('B', 1, ['roster-A']),
        makeDepAgent('C', 2, ['roster-A']),
      ],
    }
    const result = buildStepTree(steps, [], roster)
    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')

    expect(agents.map((a) => a.agentName)).toEqual(['A', 'B', 'C'])
    expect(agents.map((a) => a.gutter)).toEqual([
      ['blank', 'branch'],                // ├── A
      ['blank', 'pipe', 'fork_start'],   // │ ┌─ B
      ['blank', 'pipe', 'par_end'],      // │ └─ C
    ])
  })

  // ── Topology + workforce integration tests ──────────────────────────────

  // Pattern: Fan-in with workforce steps (the exact scenario from the bug report)
  // A(wf) → C, B(wf) → C — two workforce roots converging
  it('fan-in with workforce roots and agents', () => {
    const steps = [
      makeWorkforceStep('a', 'Python Research', 0),
      makeWorkforceStep('b', 'Rust Research', 1),
      makeWorkforceStep('c', 'Comparison', 2),
    ]
    const edges = [makeEdge('e1', 'a', 'c'), makeEdge('e2', 'b', 'c')]
    const roster: Record<string, RosterAgent[]> = {
      a: [makeRosterAgent('Researcher', 0)],
      b: [makeRosterAgent('RustResearcher', 0)],
      c: [makeRosterAgent('Comparator', 0)],
    }
    const result = buildStepTree(steps, edges, roster)

    // Steps should be connected: root_fork → par_end → corner
    expect(stepIds(result)).toEqual(['a', 'b', 'c'])
    const stepEntries = result.filter((e) => e.kind === 'step')
    expect(stepEntries.map((e) => e.gutter)).toEqual([
      ['pipe', 'fork_start'],  // │ ┌─ A (indented input)
      ['pipe', 'par_end'],     // │ └─ B (indented input)
      ['corner'],              // └── C  (base level convergence)
    ])

    // Agents should nest under their respective steps
    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')
    expect(agents).toHaveLength(3)
    expect(agents[0]!.stepId).toBe('a')
    expect(agents[1]!.stepId).toBe('b')
    expect(agents[2]!.stepId).toBe('c')
  })

  // Pattern: Diamond with mixed workforce / non-workforce
  // A(wf) → {B, C(wf)} → D
  it('diamond with mixed workforce and non-workforce', () => {
    const steps = [
      makeWorkforceStep('a', 'Leader', 0),
      makeStep('b', 'Fast Track', 1),
      makeWorkforceStep('c', 'Deep Dive', 2),
      makeStep('d', 'Merge', 3),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
      makeEdge('e3', 'b', 'd'),
      makeEdge('e4', 'c', 'd'),
    ]
    const roster: Record<string, RosterAgent[]> = {
      a: [makeRosterAgent('Manager', 0)],
      c: [makeRosterAgent('Analyst', 0), makeRosterAgent('Writer', 1)],
    }
    const result = buildStepTree(steps, edges, roster)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd'])
    const stepEntries = result.filter((e) => e.kind === 'step')
    expect(stepEntries.map((e) => e.gutter)).toEqual([
      ['branch'],                // ├── A (fork)
      ['pipe', 'fork_start'],    // │ ┌─ B
      ['pipe', 'par_end'],       // │ └─ C
      ['corner'],                // └── D (merge)
    ])

    // Manager agent under A (step gutter = branch → continuation = pipe)
    const managerAgent = result.filter((e): e is AgentEntry => e.kind === 'agent' && e.stepId === 'a')
    expect(managerAgent).toHaveLength(1)
    expect(managerAgent[0]!.gutter).toEqual(['pipe', 'corner'])

    // C's agents (step gutter = ['pipe', 'par_end'] → continuation = ['pipe', 'blank'])
    const cAgents = result.filter((e): e is AgentEntry => e.kind === 'agent' && e.stepId === 'c')
    expect(cAgents).toHaveLength(2)
  })

  // Pattern: Triple fan-in (3 roots → 1 target)
  // A → D, B → D, C → D
  it('triple fan-in from independent roots', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
    ]
    const edges = [
      makeEdge('e1', 'a', 'd'),
      makeEdge('e2', 'b', 'd'),
      makeEdge('e3', 'c', 'd'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd'])
    expect(gutters(result)).toEqual([
      ['pipe', 'fork_start'],  // │ ┌─ A (indented input)
      ['pipe', 'par_mid'],     // │ ├─ B (indented input)
      ['pipe', 'par_end'],     // │ └─ C (indented input)
      ['corner'],              // └── D  (base level convergence)
    ])
  })

  // Pattern: Fan-in where one root has a sub-chain before converging
  // A → B → D, C → D
  it('fan-in with sub-chain in root branch', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'b', 'd'),
      makeEdge('e3', 'c', 'd'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    // A and C are roots, D is convergence, B is inside A's branch
    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd'])
    expect(gutters(result)).toEqual([
      ['pipe', 'fork_start'],            // │ ┌─ A (indented input)
      ['pipe', 'pipe', 'corner'],        // │ │  └── B
      ['pipe', 'par_end'],               // │ └─ C (indented input)
      ['corner'],                         // └── D  (base level convergence)
    ])
  })

  // Pattern: Sequential workforce chain A → B → C (all workforce with agents)
  it('sequential workforce chain with agents at each level', () => {
    const steps = [
      makeWorkforceStep('a', 'Phase 1', 0),
      makeWorkforceStep('b', 'Phase 2', 1),
      makeWorkforceStep('c', 'Phase 3', 2),
    ]
    const edges = [makeEdge('e1', 'a', 'b'), makeEdge('e2', 'b', 'c')]
    const roster: Record<string, RosterAgent[]> = {
      a: [makeRosterAgent('Scout', 0)],
      b: [makeRosterAgent('Builder', 0)],
      c: [makeRosterAgent('Reviewer', 0)],
    }
    const result = buildStepTree(steps, edges, roster)

    expect(stepIds(result)).toEqual(['a', 'b', 'c'])
    const stepEntries = result.filter((e) => e.kind === 'step')
    expect(stepEntries.map((e) => e.gutter)).toEqual([
      ['branch'],   // ├── A
      ['branch'],   // ├── B
      ['corner'],   // └── C
    ])

    // Each step should have exactly 1 agent
    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')
    expect(agents).toHaveLength(3)
    expect(agents[0]!.stepId).toBe('a')
    expect(agents[0]!.gutter).toEqual(['pipe', 'corner'])    // A continuation=pipe
    expect(agents[1]!.stepId).toBe('b')
    expect(agents[1]!.gutter).toEqual(['pipe', 'corner'])    // B continuation=pipe
    expect(agents[2]!.stepId).toBe('c')
    expect(agents[2]!.gutter).toEqual(['blank', 'corner'])   // C continuation=blank (last step)
  })

  // Pattern: Workforce in parallel branch A → {B(wf), C} → D
  it('workforce step in parallel fork branch', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeWorkforceStep('b', 'B Team', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
      makeEdge('e3', 'b', 'd'),
      makeEdge('e4', 'c', 'd'),
    ]
    const roster: Record<string, RosterAgent[]> = {
      b: [makeRosterAgent('Alpha', 0), makeRosterAgent('Beta', 1)],
    }
    const result = buildStepTree(steps, edges, roster)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd'])

    // B's gutter is ['pipe', 'fork_start'], continuation = ['pipe', 'pipe']
    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')
    expect(agents).toHaveLength(2)
    expect(agents[0]!.gutter).toEqual(['pipe', 'pipe', 'branch'])
    expect(agents[1]!.gutter).toEqual(['pipe', 'pipe', 'corner'])
  })

  // Pattern: All display_order=0 (the real-world default for canvas-created steps)
  it('topology correct when all display_order=0', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 0),
      makeStep('c', 'C', 0),
    ]
    const edges = [makeEdge('e1', 'a', 'c'), makeEdge('e2', 'b', 'c')]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    // Topology should still be correct — all 3 in one component
    expect(stepIds(result)).toHaveLength(3)
    // C should be last (it's the convergence point)
    const ids = stepIds(result)
    expect(ids[ids.length - 1]).toBe('c')
    // Should have root_fork pattern (multi-root convergence)
    const stepGutters = result.filter((e) => e.kind === 'step').map((e) => e.gutter)
    expect(stepGutters[0]).toEqual(['pipe', 'fork_start'])
    expect(stepGutters[stepGutters.length - 1]).toEqual(['corner'])
  })

  // Pattern: Deeply nested forks (4 levels)
  // A → {B, C}, B → {D, E}, D → {F, G}
  it('deeply nested forks (3 levels of nesting)', () => {
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
      makeEdge('e6', 'd', 'g'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    // All 7 steps should be in the tree
    expect(stepIds(result)).toHaveLength(7)
    // A should be first, and the nesting should create progressively deeper gutters
    expect(stepIds(result)[0]).toBe('a')
    // Verify gutter depth increases with nesting
    const maxGutterDepth = Math.max(
      ...result.filter((e) => e.kind === 'step').map((e) => e.gutter.length),
    )
    expect(maxGutterDepth).toBeGreaterThanOrEqual(7) // deeply nested
  })

  // Pattern: Cross-edge within fork: A → B, A → C, B → C
  // B→C means C is reachable from B, so this isn't a true fork — it's sequential A → B → C
  it('cross-edge within fork collapses to sequential', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'a', 'c'),
      makeEdge('e3', 'b', 'c'),  // cross-edge: B → C
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    // A forks to {B, C}, but B→C means C is the merge point
    // So: A (branch), B (fork branch), then C (merge/corner)
    expect(stepIds(result)).toEqual(['a', 'b', 'c'])
  })

  // Pattern: Fan-in then fan-out — convergence gathers inputs then forks
  // A → C, B → C, C → {D, E}
  it('fan-in then fan-out', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
      makeStep('e', 'E', 4),
    ]
    const edges = [
      makeEdge('e1', 'a', 'c'),
      makeEdge('e2', 'b', 'c'),
      makeEdge('e3', 'c', 'd'),
      makeEdge('e4', 'c', 'e'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd', 'e'])
    expect(gutters(result)).toEqual([
      ['pipe', 'fork_start'],        // │ ┌─ A (indented input)
      ['pipe', 'par_end'],           // │ └─ B (indented input)
      ['branch'],                     // ├── C  (base level convergence, then forks)
      ['pipe', 'fork_start'],        // │ ┌─ D
      ['pipe', 'par_end'],           // │ └─ E
    ])
  })

  // Pattern: Wide fan-in then continuation
  // A → D, B → D, C → D, D → E
  it('wide fan-in then continuation', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
      makeStep('e', 'E', 4),
    ]
    const edges = [
      makeEdge('e1', 'a', 'd'),
      makeEdge('e2', 'b', 'd'),
      makeEdge('e3', 'c', 'd'),
      makeEdge('e4', 'd', 'e'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd', 'e'])
    expect(gutters(result)).toEqual([
      ['pipe', 'fork_start'],        // │ ┌─ A
      ['pipe', 'par_mid'],           // │ ├─ B
      ['pipe', 'par_end'],           // │ └─ C
      ['branch'],                     // ├── D (convergence)
      ['corner'],                     // └── E
    ])
  })

  // Pattern: Fan-in where both roots have sub-chains
  // A → B → E, C → D → E
  it('fan-in where both roots have sub-chains', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
      makeStep('e', 'E', 4),
    ]
    const edges = [
      makeEdge('e1', 'a', 'b'),
      makeEdge('e2', 'b', 'e'),
      makeEdge('e3', 'c', 'd'),
      makeEdge('e4', 'd', 'e'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd', 'e'])
    expect(gutters(result)).toEqual([
      ['pipe', 'fork_start'],            // │ ┌─ A
      ['pipe', 'pipe', 'corner'],        // │ │  └── B
      ['pipe', 'par_end'],               // │ └─ C
      ['pipe', 'pipe', 'corner'],        // │    └── D
      ['corner'],                         // └── E (convergence)
    ])
  })

  // Pattern: Fan-in where one root fans out before converging
  // A → {B, C} → E, D → E
  it('fan-in where one root fans out before converging', () => {
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
      makeEdge('e3', 'b', 'e'),
      makeEdge('e4', 'c', 'e'),
      makeEdge('e5', 'd', 'e'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    // A and D are roots. A fans out to B,C which merge at E. D also feeds E.
    expect(stepIds(result)).toHaveLength(5)
    // E should be last (convergence)
    const ids = stepIds(result)
    expect(ids[ids.length - 1]).toBe('e')
  })

  // Pattern: Two independent fan-in components
  // Component 1: A → C, B → C
  // Component 2: D → F, E → F
  it('parallel independent fan-in components', () => {
    const steps = [
      makeStep('a', 'A', 0),
      makeStep('b', 'B', 1),
      makeStep('c', 'C', 2),
      makeStep('d', 'D', 3),
      makeStep('e', 'E', 4),
      makeStep('f', 'F', 5),
    ]
    const edges = [
      makeEdge('e1', 'a', 'c'),
      makeEdge('e2', 'b', 'c'),
      makeEdge('e3', 'd', 'f'),
      makeEdge('e4', 'e', 'f'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    expect(stepIds(result)).toEqual(['a', 'b', 'c', 'd', 'e', 'f'])
    expect(gutters(result)).toEqual([
      ['pipe', 'fork_start'],  // │ ┌─ A
      ['pipe', 'par_end'],     // │ └─ B
      ['corner'],              // └── C
      'gap',
      ['pipe', 'fork_start'],  // │ ┌─ D
      ['pipe', 'par_end'],     // │ └─ E
      ['corner'],              // └── F
    ])
  })

  // ── Production workflow topology tests ────────────────────────────────────
  // Based on real workflow bb74cc5b — the "final boss" of topology.

  // Full production workflow: fork with skip edge + 7-agent workforce + continuation
  //
  // Step topology:
  //   AdvancedAIFact ─┬→ CreateComplexTeam ──────┐
  //                   ├→ FactEvaluator ──────────┤→ IdeaProbabilityEval → CoolnessEval
  //                   └→ [skip edge to merge] ───┘
  //
  // Agent topology within CreateComplexTeam (7 agents):
  //   FactParser ─┬→ TermExplainer → AnalogyCreator ──┐
  //               ├→ Visualizer ───────────────────────┤→ CoolSummarizer
  //               └→ ImplicationExplorer → Critic ─────┘
  it('production workflow: fork+skip+merge with 7-agent workforce', () => {
    const steps = [
      makeWorkforceStep('aif', 'Advanced AI Fact', 0),
      makeWorkforceStep('cct', 'Create Complex Team', 1),
      makeWorkforceStep('fe', 'Fact Evaluator', 2),
      makeWorkforceStep('ipe', 'Idea Probability Evaluator', 3),
      makeWorkforceStep('ce', 'Coolness Evaluator', 4),
    ]
    const edges = [
      makeEdge('e1', 'aif', 'cct'),  // root → fork branch 1
      makeEdge('e2', 'aif', 'fe'),   // root → fork branch 2
      makeEdge('e3', 'aif', 'ipe'),  // root → skip edge to merge
      makeEdge('e4', 'cct', 'ipe'),  // fork branch 1 → merge
      makeEdge('e5', 'fe', 'ipe'),   // fork branch 2 → merge
      makeEdge('e6', 'ipe', 'ce'),   // merge → continuation
    ]
    const roster: Record<string, RosterAgent[]> = {
      aif: [makeRosterAgent('DeepAIExpert', 0)],
      cct: [
        makeDepAgent('FactParser', 0, []),
        makeDepAgent('TermExplainer', 1, ['roster-FactParser']),
        makeDepAgent('AnalogyCreator', 2, ['roster-TermExplainer']),
        makeDepAgent('Visualizer', 3, ['roster-FactParser']),
        makeDepAgent('ImplicationExplorer', 4, ['roster-FactParser']),
        makeDepAgent('Critic', 5, ['roster-ImplicationExplorer']),
        makeDepAgent('CoolSummarizer', 6, ['roster-Visualizer', 'roster-AnalogyCreator', 'roster-Critic']),
      ],
      fe: [makeRosterAgent('FactChecker', 0)],
      ipe: [makeRosterAgent('Evaluator', 0)],
      ce: [makeRosterAgent('CoolnessEvaluator', 0)],
    }
    const result = buildStepTree(steps, edges, roster)

    // ── Step verification ──
    expect(stepIds(result)).toEqual(['aif', 'cct', 'fe', 'ipe', 'ce'])
    const stepEntries = result.filter((e) => e.kind === 'step')
    expect(stepEntries.map((e) => e.gutter)).toEqual([
      ['branch'],                // ├── AdvancedAIFact (fork point)
      ['pipe', 'fork_start'],    // │ ┌─ CreateComplexTeam (branch 1)
      ['pipe', 'par_end'],       // │ └─ FactEvaluator (branch 2)
      ['branch'],                // ├── IdeaProbabilityEval (merge point)
      ['corner'],                // └── CoolnessEval (tail)
    ])

    // ── Agent count verification ──
    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')
    expect(agents).toHaveLength(11) // 1 + 7 + 1 + 1 + 1

    // ── Agent parent verification ──
    expect(agents.filter((a) => a.stepId === 'aif').map((a) => a.agentName)).toEqual(['DeepAIExpert'])
    expect(agents.filter((a) => a.stepId === 'cct').map((a) => a.agentName)).toEqual([
      'FactParser', 'TermExplainer', 'AnalogyCreator', 'Visualizer', 'ImplicationExplorer', 'Critic', 'CoolSummarizer',
    ])
    expect(agents.filter((a) => a.stepId === 'fe').map((a) => a.agentName)).toEqual(['FactChecker'])
    expect(agents.filter((a) => a.stepId === 'ipe').map((a) => a.agentName)).toEqual(['Evaluator'])
    expect(agents.filter((a) => a.stepId === 'ce').map((a) => a.agentName)).toEqual(['CoolnessEvaluator'])

    // ── Single-agent step gutter verification ──
    // DeepAIExpert under fork-point (branch → continuation=pipe)
    expect(agents.find((a) => a.agentName === 'DeepAIExpert')!.gutter).toEqual(['pipe', 'corner'])
    // FactChecker under par_end (continuation=['pipe','blank'])
    expect(agents.find((a) => a.agentName === 'FactChecker')!.gutter).toEqual(['pipe', 'blank', 'corner'])
    // Evaluator under merge point (branch → continuation=pipe)
    expect(agents.find((a) => a.agentName === 'Evaluator')!.gutter).toEqual(['pipe', 'corner'])
    // CoolnessEvaluator under tail (corner → continuation=blank)
    expect(agents.find((a) => a.agentName === 'CoolnessEvaluator')!.gutter).toEqual(['blank', 'corner'])

    // ── 7-agent topology gutter verification ──
    // CreateComplexTeam gutter is ['pipe', 'fork_start'], continuation = ['pipe', 'pipe']
    const cctAgents = agents.filter((a) => a.stepId === 'cct')
    // FactParser is the root agent → gets branch (more follow)
    expect(cctAgents[0]!.gutter).toEqual(['pipe', 'pipe', 'branch'])
    // CoolSummarizer is the merge/last agent → gets corner
    expect(cctAgents[6]!.gutter).toEqual(['pipe', 'pipe', 'corner'])
  })

  // Agent topology: 3-branch fork with sub-chains merging into one
  // FactParser → {TermExplainer→AnalogyCreator, Visualizer, ImplicationExplorer→Critic} → CoolSummarizer
  it('agent topology: 3-branch fork with sub-chains and merge', () => {
    const steps = [makeWorkforceStep('wf', 'Team', 0)]
    const roster: Record<string, RosterAgent[]> = {
      wf: [
        makeDepAgent('FactParser', 0, []),
        makeDepAgent('TermExplainer', 1, ['roster-FactParser']),
        makeDepAgent('AnalogyCreator', 2, ['roster-TermExplainer']),
        makeDepAgent('Visualizer', 3, ['roster-FactParser']),
        makeDepAgent('ImplicationExplorer', 4, ['roster-FactParser']),
        makeDepAgent('Critic', 5, ['roster-ImplicationExplorer']),
        makeDepAgent('CoolSummarizer', 6, ['roster-Visualizer', 'roster-AnalogyCreator', 'roster-Critic']),
      ],
    }
    const result = buildStepTree(steps, [], roster)
    const agents = result.filter((e): e is AgentEntry => e.kind === 'agent')

    // All 7 agents should be present
    expect(agents).toHaveLength(7)
    expect(agents.map((a) => a.agentName)).toEqual([
      'FactParser', 'TermExplainer', 'AnalogyCreator', 'Visualizer', 'ImplicationExplorer', 'Critic', 'CoolSummarizer',
    ])

    // FactParser is root/fork → branch
    expect(agents[0]!.gutter).toEqual(['blank', 'branch'])
    // CoolSummarizer is merge/last → corner
    expect(agents[6]!.gutter).toEqual(['blank', 'corner'])
    // Middle agents should be indented (fork branches)
    // TermExplainer starts first branch
    expect(agents[1]!.gutter).toEqual(['blank', 'pipe', 'fork_start'])
  })

  // Pattern: Edge referencing hidden-mode step
  // A → ctx(context) → B — context step filtered, A and B become separate
  it('edges through hidden-mode steps are dropped', () => {
    const steps = [
      makeStep('a', 'A', 0),
      { ...makeStep('ctx', 'Context', 1), execution_mode: 'context' as const },
      makeStep('b', 'B', 2),
    ]
    const edges = [
      makeEdge('e1', 'a', 'ctx'),
      makeEdge('e2', 'ctx', 'b'),
    ]
    const result = buildStepTree(steps, edges, NO_ROSTER)

    // Context step is hidden, edges to/from it are dropped
    // A and B should be separate components with a gap
    expect(stepIds(result)).toEqual(['a', 'b'])
    expect(gutters(result)).toEqual([
      ['corner'],
      'gap',
      ['corner'],
    ])
  })
})
