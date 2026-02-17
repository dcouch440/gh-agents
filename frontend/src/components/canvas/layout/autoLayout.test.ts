import { describe, it, expect } from 'vitest'
import { computeAutoLayout, classifyStep, topologicalSort, buildTieredTower, computeAgentTiers } from './autoLayout'
import type { NodeRole } from './autoLayout'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import type { StepNodeLookups, RosterAgentInfo } from '../mappers/types'
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

const makeAgent = (id: string, name: string, depsOn: string[] = []): RosterAgentInfo => ({
  id,
  name,
  child_step_id: `cs-${id}`,
  role_description: '',
  depends_on: depsOn,
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
// computeAgentTiers
// ============================================================================

describe('computeAgentTiers', () => {
  it('assigns tier 0 to agents with no dependencies', () => {
    const roster: RosterAgentInfo[] = [
      makeAgent('r1', 'Agent A'),
      makeAgent('r2', 'Agent B'),
    ]
    const tierMap = computeAgentTiers(roster)
    expect(tierMap.get('r1')).toBe(0)
    expect(tierMap.get('r2')).toBe(0)
  })

  it('assigns tier 1 to agents depending on tier-0 agents', () => {
    const roster: RosterAgentInfo[] = [
      makeAgent('r1', 'Designer A'),
      makeAgent('r2', 'Designer B'),
      makeAgent('r3', 'Judge', ['r1', 'r2']),
    ]
    const tierMap = computeAgentTiers(roster)
    expect(tierMap.get('r1')).toBe(0)
    expect(tierMap.get('r2')).toBe(0)
    expect(tierMap.get('r3')).toBe(1)
  })

  it('handles linear chain: A → B → C', () => {
    const roster: RosterAgentInfo[] = [
      makeAgent('r1', 'A'),
      makeAgent('r2', 'B', ['r1']),
      makeAgent('r3', 'C', ['r2']),
    ]
    const tierMap = computeAgentTiers(roster)
    expect(tierMap.get('r1')).toBe(0)
    expect(tierMap.get('r2')).toBe(1)
    expect(tierMap.get('r3')).toBe(2)
  })

  it('ignores depends_on references to inactive agents', () => {
    const roster: RosterAgentInfo[] = [
      { id: 'r1', name: 'A', child_step_id: null, role_description: '', depends_on: [] },
      makeAgent('r2', 'B', ['r1']),
    ]
    const tierMap = computeAgentTiers(roster)
    // r1 is inactive (no child_step_id), so r2 has no active deps → tier 0
    expect(tierMap.get('r2')).toBe(0)
    expect(tierMap.has('r1')).toBe(false)
  })

  it('handles diamond: 2 parallel → 1 dependent', () => {
    const roster: RosterAgentInfo[] = [
      makeAgent('r1', 'A'),
      makeAgent('r2', 'B'),
      makeAgent('r3', 'C', ['r1', 'r2']),
      makeAgent('r4', 'D', ['r3']),
    ]
    const tierMap = computeAgentTiers(roster)
    expect(tierMap.get('r1')).toBe(0)
    expect(tierMap.get('r2')).toBe(0)
    expect(tierMap.get('r3')).toBe(1)
    expect(tierMap.get('r4')).toBe(2)
  })
})

// ============================================================================
// buildTieredTower
// ============================================================================

describe('buildTieredTower', () => {
  it('returns empty for step with no roster', () => {
    const lookups = emptyLookups()
    expect(buildTieredTower('s1', lookups)).toEqual([])
  })

  it('groups parallel agents into tier 0', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        's1': [
          makeAgent('r1', 'Agent A'),
          makeAgent('r2', 'Agent B'),
        ],
      },
    })
    const tiers = buildTieredTower('s1', lookups)
    expect(tiers).toHaveLength(1)
    expect(tiers[0]!.tier).toBe(0)
    expect(tiers[0]!.entries).toHaveLength(2)
  })

  it('separates dependent agents into different tiers', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        's1': [
          makeAgent('r1', 'Designer A'),
          makeAgent('r2', 'Designer B'),
          makeAgent('r3', 'Judge', ['r1', 'r2']),
        ],
      },
    })
    const tiers = buildTieredTower('s1', lookups)
    expect(tiers).toHaveLength(2)
    expect(tiers[0]!.tier).toBe(0)
    expect(tiers[0]!.entries).toHaveLength(2)
    expect(tiers[1]!.tier).toBe(1)
    expect(tiers[1]!.entries).toHaveLength(1)
  })

  it('pairs agents with their documents', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        's1': [makeAgent('r1', 'Agent A')],
      },
      documentDefsByStep: {
        's1': [{ id: 'd1', name: 'Doc A', document_id: null, agent_roster_entry_id: 'r1' }],
      },
    })
    const tiers = buildTieredTower('s1', lookups)
    expect(tiers[0]!.entries[0]!.documentNodeId).toBe('doc-artifact-d1')
  })

  it('skips agents without child_step_id', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        's1': [
          { id: 'r1', name: 'Agent A', child_step_id: null, role_description: '', depends_on: [] },
        ],
      },
    })
    expect(buildTieredTower('s1', lookups)).toEqual([])
  })

  it('includes unassigned documents in tier 0', () => {
    const lookups = emptyLookups({
      rosterByStep: { 's1': [] },
      documentDefsByStep: {
        's1': [{ id: 'd1', name: 'Orphan Doc', document_id: null, agent_roster_entry_id: null }],
      },
    })
    const tiers = buildTieredTower('s1', lookups)
    expect(tiers).toHaveLength(1)
    expect(tiers[0]!.entries[0]!.agentNodeId).toBe('')
    expect(tiers[0]!.entries[0]!.documentNodeId).toBe('doc-artifact-d1')
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

  it('places parallel agents side-by-side in same tier', () => {
    const steps = [makeStep({ id: 'p1', execution_mode: 'workforce' })]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
      rosterByStep: {
        'p1': [
          makeAgent('r1', 'Agent A'),
          makeAgent('r2', 'Agent B'),
          makeAgent('r3', 'Agent C'),
        ],
      },
    })

    const result = computeAutoLayout(steps, [], lookups)

    const agent1 = result.get('agent-artifact-r1')!
    const agent2 = result.get('agent-artifact-r2')!
    const agent3 = result.get('agent-artifact-r3')!

    // All at same Y (same tier)
    expect(agent1.y).toBe(agent2.y)
    expect(agent2.y).toBe(agent3.y)

    // Spread horizontally
    expect(agent1.x).toBeLessThan(agent2.x)
    expect(agent2.x).toBeLessThan(agent3.x)

    // All above protocol
    const protocolY = result.get('p1')!.y
    expect(agent1.y).toBeLessThan(protocolY)
  })

  it('places dependent agent in higher tier (further from protocol)', () => {
    const steps = [makeStep({ id: 'p1', execution_mode: 'workforce' })]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
      rosterByStep: {
        'p1': [
          makeAgent('r1', 'Designer A'),
          makeAgent('r2', 'Designer B'),
          makeAgent('r3', 'Judge', ['r1', 'r2']),
        ],
      },
    })

    const result = computeAutoLayout(steps, [], lookups)

    const designerA = result.get('agent-artifact-r1')!
    const designerB = result.get('agent-artifact-r2')!
    const judge = result.get('agent-artifact-r3')!

    // Designers are at same Y (tier 0)
    expect(designerA.y).toBe(designerB.y)
    // Judge is further from protocol (higher tier → more negative Y)
    expect(judge.y).toBeLessThan(designerA.y)

    // Designers side by side
    expect(designerA.x).toBeLessThan(designerB.x)
  })

  it('handles diamond pattern: 2 parallel → 1 judge', () => {
    const steps = [makeStep({ id: 'p1', execution_mode: 'workforce' })]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
      rosterByStep: {
        'p1': [
          makeAgent('r1', 'A'),
          makeAgent('r2', 'B'),
          makeAgent('r3', 'C', ['r1', 'r2']),
        ],
      },
      documentDefsByStep: {
        'p1': [
          { id: 'd1', name: 'Doc A', document_id: null, agent_roster_entry_id: 'r1' },
          { id: 'd3', name: 'Doc C', document_id: null, agent_roster_entry_id: 'r3' },
        ],
      },
    })

    const result = computeAutoLayout(steps, [], lookups)

    const agentA = result.get('agent-artifact-r1')!
    const agentB = result.get('agent-artifact-r2')!
    const agentC = result.get('agent-artifact-r3')!
    const docA = result.get('doc-artifact-d1')!
    const docC = result.get('doc-artifact-d3')!

    // A and B at same tier
    expect(agentA.y).toBe(agentB.y)
    // C at higher tier
    expect(agentC.y).toBeLessThan(agentA.y)

    // Docs above their agents
    expect(docA.y).toBeLessThan(agentA.y)
    expect(docC.y).toBeLessThan(agentC.y)

    // Doc A same x as agent A (both same width)
    expect(docA.x).toBe(agentA.x)
  })

  it('handles linear chain: A → B → C', () => {
    const steps = [makeStep({ id: 'p1', execution_mode: 'workforce' })]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
      rosterByStep: {
        'p1': [
          makeAgent('r1', 'A'),
          makeAgent('r2', 'B', ['r1']),
          makeAgent('r3', 'C', ['r2']),
        ],
      },
    })

    const result = computeAutoLayout(steps, [], lookups)

    const agentA = result.get('agent-artifact-r1')!
    const agentB = result.get('agent-artifact-r2')!
    const agentC = result.get('agent-artifact-r3')!

    // Each in a separate tier, stacking upward
    expect(agentB.y).toBeLessThan(agentA.y)
    expect(agentC.y).toBeLessThan(agentB.y)
  })

  it('positions documents above their agents', () => {
    const steps = [makeStep({ id: 'p1', execution_mode: 'workforce' })]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
      rosterByStep: {
        'p1': [makeAgent('r1', 'Agent A')],
      },
      documentDefsByStep: {
        'p1': [{ id: 'd1', name: 'Doc A', document_id: null, agent_roster_entry_id: 'r1' }],
      },
    })

    const result = computeAutoLayout(steps, [], lookups)

    const agentPos = result.get('agent-artifact-r1')!
    const docPos = result.get('doc-artifact-d1')!

    // Document is above agent
    expect(docPos.y).toBeLessThan(agentPos.y)
    expect(docPos.y).toBe(agentPos.y - DOCUMENT_NODE.DEFAULT_HEIGHT - AUTO_LAYOUT.DOC_GAP)
    // Same x-coordinate (agent and doc are same width)
    expect(docPos.x).toBe(agentPos.x)
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
          makeAgent('r1', 'Agent A'),
          makeAgent('r2', 'Agent B'),
          makeAgent('r3', 'Agent C'),
        ],
        'p2': [
          makeAgent('r4', 'Agent D'),
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

    // All agents present
    expect(result.has('agent-artifact-r1')).toBe(true)
    expect(result.has('agent-artifact-r2')).toBe(true)
    expect(result.has('agent-artifact-r3')).toBe(true)
    expect(result.has('agent-artifact-r4')).toBe(true)
  })

  it('computes column width based on widest tier', () => {
    const steps = [
      makeStep({ id: 'input', execution_mode: 'input' }),
      makeStep({ id: 'p1', execution_mode: 'workforce' }),
    ]
    const edges = [makeEdge('input', 'p1')]
    const lookups = emptyLookups({
      protocolsByStep: new Map([['p1', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
      rosterByStep: {
        'p1': [
          makeAgent('r1', 'Agent A'),
          makeAgent('r2', 'Agent B'),
        ],
      },
    })

    const result = computeAutoLayout(steps, edges, lookups)

    const inputPos = result.get('input')!
    const p1Pos = result.get('p1')!

    // Column width = widest tier width (2 agents side-by-side)
    const inputWidth = NODE_DIMENSIONS[CanvasNodeKind.INPUT].defaultWidth
    const tierWidth = 2 * AGENT_DEFAULTS.DEFAULT_WIDTH + AUTO_LAYOUT.TIER_AGENT_GAP
    const protocolWidth = NODE_DIMENSIONS[CanvasNodeKind.PROTOCOL].defaultWidth
    const columnWidth = Math.max(protocolWidth, tierWidth)

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
    expect(result.size).toBe(1)
  })
})
