import { describe, it, expect } from 'vitest'
import { computeAutoLayout, classifyStep, topologicalSort, buildTower } from './autoLayout'
import type { NodeRole, TowerEntry } from './autoLayout'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import type { StepNodeLookups } from '../mappers/types'
import { AUTO_LAYOUT } from './autoLayoutConfig'
import { NODE_DIMENSIONS } from '../nodeDimensions'
import { CanvasNodeKind } from '../canvasKinds'
import { AGENT_DEFAULTS } from '../DynamicNode/archetypes'
import { DOCUMENT_NODE } from '../DocumentNode'

// ============================================================================
// Helpers
// ============================================================================

const makeStep = (overrides: Partial<WorkflowStep> & { id: string }): WorkflowStep => ({
  workflow_id: 'wf-1',
  agent_id: '',
  execution_mode: 'single',
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
  ...overrides,
})

const makeEdge = (from: string, to: string): WorkflowStepEdge => ({
  id: `edge-${from}-${to}`,
  from_step_id: from,
  to_step_id: to,
})

const emptyLookups = (overrides?: Partial<StepNodeLookups>): StepNodeLookups => ({
  agents: new Map(),
  outputSchemas: new Map(),
  stepNames: new Map(),
  edges: [],
  toolsByAgent: new Map(),
  protocolsByStep: new Map(),
  documentDefsByStep: {},
  rosterByStep: {},
  notesByStep: {},
  documentContentByDefId: {},
  protocolGroups: new Map(),
  ...overrides,
})

// ============================================================================
// classifyStep
// ============================================================================

describe('classifyStep', () => {
  it('classifies input steps', () => {
    const step = makeStep({ id: 's1', execution_mode: 'input' })
    expect(classifyStep(step, new Map())).toBe('input' satisfies NodeRole)
  })

  it('classifies context steps', () => {
    const step = makeStep({ id: 's1', execution_mode: 'context' })
    expect(classifyStep(step, new Map())).toBe('context' satisfies NodeRole)
  })

  it('classifies workforce steps as protocol', () => {
    const step = makeStep({ id: 's1', execution_mode: 'workforce' })
    expect(classifyStep(step, new Map())).toBe('protocol' satisfies NodeRole)
  })

  it('classifies room steps as protocol', () => {
    const step = makeStep({ id: 's1', execution_mode: 'room' })
    expect(classifyStep(step, new Map())).toBe('protocol' satisfies NodeRole)
  })

  it('classifies steps in protocolsByStep as protocol', () => {
    const step = makeStep({ id: 's1', execution_mode: 'single' })
    const protocols = new Map([['s1', { protocol_type: 'decomp', name: 'P1', portNames: [] }]])
    expect(classifyStep(step, protocols)).toBe('protocol' satisfies NodeRole)
  })

  it('classifies regular steps', () => {
    const step = makeStep({ id: 's1', execution_mode: 'single' })
    expect(classifyStep(step, new Map())).toBe('step' satisfies NodeRole)
  })
})

// ============================================================================
// topologicalSort
// ============================================================================

describe('topologicalSort', () => {
  it('returns single node', () => {
    const result = topologicalSort(new Set(['a']), [])
    expect(result).toEqual(['a'])
  })

  it('sorts a simple chain', () => {
    const ids = new Set(['a', 'b', 'c'])
    const edges = [makeEdge('a', 'b'), makeEdge('b', 'c')]
    const result = topologicalSort(ids, edges)
    expect(result).toEqual(['a', 'b', 'c'])
  })

  it('handles diamond graph', () => {
    const ids = new Set(['a', 'b', 'c', 'd'])
    const edges = [makeEdge('a', 'b'), makeEdge('a', 'c'), makeEdge('b', 'd'), makeEdge('c', 'd')]
    const result = topologicalSort(ids, edges)
    expect(result.indexOf('a')).toBeLessThan(result.indexOf('b'))
    expect(result.indexOf('a')).toBeLessThan(result.indexOf('c'))
    expect(result.indexOf('b')).toBeLessThan(result.indexOf('d'))
    expect(result.indexOf('c')).toBeLessThan(result.indexOf('d'))
  })

  it('ignores edges referencing nodes outside the set', () => {
    const ids = new Set(['a', 'b'])
    const edges = [makeEdge('a', 'b'), makeEdge('b', 'z')]
    const result = topologicalSort(ids, edges)
    expect(result).toEqual(['a', 'b'])
  })

  it('handles disconnected nodes', () => {
    const ids = new Set(['a', 'b'])
    const result = topologicalSort(ids, [])
    expect(result).toHaveLength(2)
    expect(result).toContain('a')
    expect(result).toContain('b')
  })
})

// ============================================================================
// buildTower
// ============================================================================

describe('buildTower', () => {
  it('returns empty for step with no roster', () => {
    const lookups = emptyLookups()
    expect(buildTower('s1', lookups)).toEqual([])
  })

  it('builds entries for agents with child steps', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        's1': [
          { id: 'r1', name: 'Agent A', child_step_id: 'cs1', role_description: '', depends_on: [] },
          { id: 'r2', name: 'Agent B', child_step_id: 'cs2', role_description: '', depends_on: [] },
        ],
      },
      documentDefsByStep: {
        's1': [
          { id: 'd1', name: 'Doc A', document_id: null, agent_roster_entry_id: 'r1' },
        ],
      },
    })
    const tower = buildTower('s1', lookups)
    expect(tower).toHaveLength(2)
    expect(tower[0]).toEqual({ agentNodeId: 'agent-artifact-r1', documentNodeId: 'doc-artifact-d1' } satisfies TowerEntry)
    expect(tower[1]).toEqual({ agentNodeId: 'agent-artifact-r2', documentNodeId: null } satisfies TowerEntry)
  })

  it('skips agents without child_step_id', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        's1': [
          { id: 'r1', name: 'Agent A', child_step_id: null, role_description: '', depends_on: [] },
        ],
      },
    })
    expect(buildTower('s1', lookups)).toEqual([])
  })

  it('includes unassigned documents', () => {
    const lookups = emptyLookups({
      rosterByStep: { 's1': [] },
      documentDefsByStep: {
        's1': [
          { id: 'd1', name: 'Orphan Doc', document_id: null, agent_roster_entry_id: null },
        ],
      },
    })
    const tower = buildTower('s1', lookups)
    expect(tower).toHaveLength(1)
    expect(tower[0]!.agentNodeId).toBe('')
    expect(tower[0]!.documentNodeId).toBe('doc-artifact-d1')
  })
})

// ============================================================================
// computeAutoLayout
// ============================================================================

describe('computeAutoLayout', () => {
  it('returns empty map for empty workflow', () => {
    const result = computeAutoLayout([], [], emptyLookups())
    expect(result.size).toBe(0)
  })

  it('positions a single protocol on the spine', () => {
    const steps = [makeStep({ id: 'p1', execution_mode: 'workforce' })]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
    })
    const result = computeAutoLayout(steps, [], lookups)

    const pos = result.get('p1')
    expect(pos).toBeDefined()
    expect(pos!.y).toBe(AUTO_LAYOUT.SPINE_Y)
  })

  it('positions input before context before protocol', () => {
    const steps = [
      makeStep({ id: 'input', execution_mode: 'input' }),
      makeStep({ id: 'ctx', execution_mode: 'context' }),
      makeStep({ id: 'p1', execution_mode: 'workforce' }),
    ]
    const edges = [makeEdge('input', 'ctx'), makeEdge('ctx', 'p1')]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
    })

    const result = computeAutoLayout(steps, edges, lookups)

    const inputPos = result.get('input')!
    const ctxPos = result.get('ctx')!
    const p1Pos = result.get('p1')!

    expect(inputPos.x).toBeLessThan(ctxPos.x)
    expect(ctxPos.x).toBeLessThan(p1Pos.x)
    expect(inputPos.y).toBe(AUTO_LAYOUT.SPINE_Y)
    expect(ctxPos.y).toBe(AUTO_LAYOUT.SPINE_Y)
    expect(p1Pos.y).toBe(AUTO_LAYOUT.SPINE_Y)
  })

  it('positions agents in tower above protocol', () => {
    const steps = [makeStep({ id: 'p1', execution_mode: 'workforce' })]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
      rosterByStep: {
        'p1': [
          { id: 'r1', name: 'Agent A', child_step_id: 'cs1', role_description: '', depends_on: [] },
          { id: 'r2', name: 'Agent B', child_step_id: 'cs2', role_description: '', depends_on: [] },
          { id: 'r3', name: 'Agent C', child_step_id: 'cs3', role_description: '', depends_on: [] },
        ],
      },
    })

    const result = computeAutoLayout(steps, [], lookups)

    const protocolY = result.get('p1')!.y
    const agent1 = result.get('agent-artifact-r1')
    const agent2 = result.get('agent-artifact-r2')
    const agent3 = result.get('agent-artifact-r3')

    expect(agent1).toBeDefined()
    expect(agent2).toBeDefined()
    expect(agent3).toBeDefined()

    // All agents above protocol
    expect(agent1!.y).toBeLessThan(protocolY)
    expect(agent2!.y).toBeLessThan(protocolY)
    expect(agent3!.y).toBeLessThan(protocolY)

    // Agents stack upward (each further from protocol)
    expect(agent2!.y).toBeLessThan(agent1!.y)
    expect(agent3!.y).toBeLessThan(agent2!.y)
  })

  it('positions documents to the right of their agents', () => {
    const steps = [makeStep({ id: 'p1', execution_mode: 'workforce' })]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
      rosterByStep: {
        'p1': [
          { id: 'r1', name: 'Agent A', child_step_id: 'cs1', role_description: '', depends_on: [] },
        ],
      },
      documentDefsByStep: {
        'p1': [
          { id: 'd1', name: 'Doc A', document_id: null, agent_roster_entry_id: 'r1' },
        ],
      },
    })

    const result = computeAutoLayout(steps, [], lookups)

    const agentPos = result.get('agent-artifact-r1')!
    const docPos = result.get('doc-artifact-d1')!

    // Document is to the right of agent
    expect(docPos.x).toBe(agentPos.x + AGENT_DEFAULTS.DEFAULT_WIDTH + AUTO_LAYOUT.DOC_GAP)
    // Same y-coordinate (same row)
    expect(docPos.y).toBe(agentPos.y)
  })

  it('positions notes below protocol', () => {
    const steps = [makeStep({ id: 'p1', execution_mode: 'workforce' })]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
      notesByStep: { 'p1': 'Some notes content' },
    })

    const result = computeAutoLayout(steps, [], lookups)

    const protocolPos = result.get('p1')!
    const notesPos = result.get('notes-p1')

    expect(notesPos).toBeDefined()
    expect(notesPos!.y).toBeGreaterThan(protocolPos.y)
    expect(notesPos!.y).toBe(
      protocolPos.y + NODE_DIMENSIONS[CanvasNodeKind.PROTOCOL].defaultHeight + AUTO_LAYOUT.NOTES_GAP,
    )
  })

  it('handles two protocols with different tower heights', () => {
    const steps = [
      makeStep({ id: 'p1', execution_mode: 'workforce' }),
      makeStep({ id: 'p2', execution_mode: 'workforce' }),
    ]
    const edges = [makeEdge('p1', 'p2')]
    const lookups = emptyLookups({
      protocolsByStep: new Map([
        ['p1', { protocol_type: 'workforce', name: 'Team 1', portNames: [] }],
        ['p2', { protocol_type: 'workforce', name: 'Team 2', portNames: [] }],
      ]),
      rosterByStep: {
        'p1': [
          { id: 'r1', name: 'Agent A', child_step_id: 'cs1', role_description: '', depends_on: [] },
          { id: 'r2', name: 'Agent B', child_step_id: 'cs2', role_description: '', depends_on: [] },
          { id: 'r3', name: 'Agent C', child_step_id: 'cs3', role_description: '', depends_on: [] },
        ],
        'p2': [
          { id: 'r4', name: 'Agent D', child_step_id: 'cs4', role_description: '', depends_on: [] },
        ],
      },
    })

    const result = computeAutoLayout(steps, edges, lookups)

    const p1Pos = result.get('p1')!
    const p2Pos = result.get('p2')!

    // P2 is to the right of P1
    expect(p2Pos.x).toBeGreaterThan(p1Pos.x)

    // Both on the spine
    expect(p1Pos.y).toBe(AUTO_LAYOUT.SPINE_Y)
    expect(p2Pos.y).toBe(AUTO_LAYOUT.SPINE_Y)

    // P1 has 3 agents, P2 has 1 — both present
    expect(result.has('agent-artifact-r1')).toBe(true)
    expect(result.has('agent-artifact-r2')).toBe(true)
    expect(result.has('agent-artifact-r3')).toBe(true)
    expect(result.has('agent-artifact-r4')).toBe(true)
  })

  it('computes correct column width for protocol with tower', () => {
    const steps = [
      makeStep({ id: 'input', execution_mode: 'input' }),
      makeStep({ id: 'p1', execution_mode: 'workforce' }),
    ]
    const edges = [makeEdge('input', 'p1')]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
      rosterByStep: {
        'p1': [
          { id: 'r1', name: 'Agent A', child_step_id: 'cs1', role_description: '', depends_on: [] },
        ],
      },
      documentDefsByStep: {
        'p1': [
          { id: 'd1', name: 'Doc A', document_id: null, agent_roster_entry_id: 'r1' },
        ],
      },
    })

    const result = computeAutoLayout(steps, edges, lookups)

    const inputPos = result.get('input')!
    const p1Pos = result.get('p1')!

    // The input column is INPUT_WIDTH, then SPINE_GAP, then protocol column starts
    const inputWidth = NODE_DIMENSIONS[CanvasNodeKind.INPUT].defaultWidth
    const towerWidth = AGENT_DEFAULTS.DEFAULT_WIDTH + AUTO_LAYOUT.DOC_GAP + DOCUMENT_NODE.DEFAULT_WIDTH
    const protocolWidth = NODE_DIMENSIONS[CanvasNodeKind.PROTOCOL].defaultWidth
    const columnWidth = Math.max(protocolWidth, towerWidth)

    // Input starts at 0, protocol column starts at inputWidth + SPINE_GAP
    expect(inputPos.x).toBe(0)
    const expectedProtocolX = inputWidth + AUTO_LAYOUT.SPINE_GAP + (columnWidth - protocolWidth) / 2
    expect(p1Pos.x).toBe(expectedProtocolX)
  })

  it('does not create notes nodes when no notes content', () => {
    const steps = [makeStep({ id: 'p1', execution_mode: 'workforce' })]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
      notesByStep: {},
    })

    const result = computeAutoLayout(steps, [], lookups)
    expect(result.has('notes-p1')).toBe(false)
  })

  it('handles protocol with zero agents', () => {
    const steps = [makeStep({ id: 'p1', execution_mode: 'workforce' })]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
    })

    const result = computeAutoLayout(steps, [], lookups)
    expect(result.get('p1')).toBeDefined()
    // Only the protocol node, no agents
    expect(result.size).toBe(1)
  })
})
