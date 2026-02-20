import { describe, it, expect } from 'vitest'
import { computeTowerPositions, computeAllTowerPositions, buildTieredTower, computeAgentTiers } from './autoLayout'
import type { ProtocolDimensions, NodeDimensions } from './autoLayout'
import type { StepNodeLookups, RosterAgentInfo } from '../mappers/types'
import { TOWER_LAYOUT } from './autoLayoutConfig'
import { AGENT_DEFAULTS } from '../CanvasNode/registry'

// ============================================================================
// Helpers
// ============================================================================

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
  rosterByStep: {},
  protocolGroups: new Map(),
  ...overrides,
})

const defaultProtocol: ProtocolDimensions = { x: 0, y: 0, width: 560 }

const emptyMeasured = new Map<string, NodeDimensions>()

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
})

// ============================================================================
// computeTowerPositions
// ============================================================================

describe('computeTowerPositions', () => {
  it('returns empty map for protocol with no roster', () => {
    const lookups = emptyLookups()
    const result = computeTowerPositions('p1', defaultProtocol, lookups, emptyMeasured)
    expect(result.size).toBe(0)
  })

  it('places parallel agents side-by-side above protocol', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        'p1': [
          makeAgent('r1', 'Agent A'),
          makeAgent('r2', 'Agent B'),
          makeAgent('r3', 'Agent C'),
        ],
      },
    })

    const result = computeTowerPositions('p1', defaultProtocol, lookups, emptyMeasured)

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
    expect(agent1.y).toBeLessThan(defaultProtocol.y)
  })

  it('places dependent agent in higher tier (further from protocol)', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        'p1': [
          makeAgent('r1', 'Designer A'),
          makeAgent('r2', 'Designer B'),
          makeAgent('r3', 'Judge', ['r1', 'r2']),
        ],
      },
    })

    const result = computeTowerPositions('p1', defaultProtocol, lookups, emptyMeasured)

    const designerA = result.get('agent-artifact-r1')!
    const designerB = result.get('agent-artifact-r2')!
    const judge = result.get('agent-artifact-r3')!

    // Designers at same Y
    expect(designerA.y).toBe(designerB.y)
    // Judge further from protocol (more negative Y)
    expect(judge.y).toBeLessThan(designerA.y)
    // Designers side by side
    expect(designerA.x).toBeLessThan(designerB.x)
  })

  it('centers agents on protocol width', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        'p1': [makeAgent('r1', 'Solo Agent')],
      },
    })

    const protocol: ProtocolDimensions = { x: 100, y: 200, width: 600 }
    const result = computeTowerPositions('p1', protocol, lookups, emptyMeasured)

    const agent = result.get('agent-artifact-r1')!
    const agentCenter = agent.x + AGENT_DEFAULTS.DEFAULT_WIDTH / 2
    const protocolCenter = protocol.x + protocol.width / 2

    expect(agentCenter).toBe(protocolCenter)
  })

  it('uses measured dimensions when available', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        'p1': [
          makeAgent('r1', 'Small Agent'),
          makeAgent('r2', 'Tall Agent'),
        ],
      },
    })

    const measured = new Map<string, NodeDimensions>([
      ['agent-artifact-r1', { width: 400, height: 300 }],
      ['agent-artifact-r2', { width: 400, height: 600 }],
    ])

    const result = computeTowerPositions('p1', defaultProtocol, lookups, measured)

    const agent1 = result.get('agent-artifact-r1')!
    const agent2 = result.get('agent-artifact-r2')!

    // Both at same Y (tallest in tier determines position)
    expect(agent1.y).toBe(agent2.y)

    // Y should account for the tallest agent (600px), not default
    const expectedY = defaultProtocol.y - TOWER_LAYOUT.TOWER_START_GAP - 600
    expect(agent1.y).toBe(expectedY)
  })

  it('uses measured width for horizontal spacing', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        'p1': [
          makeAgent('r1', 'Wide Agent'),
          makeAgent('r2', 'Narrow Agent'),
        ],
      },
    })

    const measured = new Map<string, NodeDimensions>([
      ['agent-artifact-r1', { width: 500, height: 360 }],
      ['agent-artifact-r2', { width: 300, height: 360 }],
    ])

    const result = computeTowerPositions('p1', defaultProtocol, lookups, measured)

    const agent1 = result.get('agent-artifact-r1')!
    const agent2 = result.get('agent-artifact-r2')!

    // Agent2 should start after agent1's measured width + gap
    const expectedAgent2X = agent1.x + 500 + TOWER_LAYOUT.TIER_AGENT_GAP
    expect(agent2.x).toBe(expectedAgent2X)
  })

  it('handles linear chain with measured heights', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        'p1': [
          makeAgent('r1', 'A'),
          makeAgent('r2', 'B', ['r1']),
        ],
      },
    })

    const measured = new Map<string, NodeDimensions>([
      ['agent-artifact-r1', { width: 420, height: 400 }],
      ['agent-artifact-r2', { width: 420, height: 500 }],
    ])

    const result = computeTowerPositions('p1', defaultProtocol, lookups, measured)

    const agentA = result.get('agent-artifact-r1')!
    const agentB = result.get('agent-artifact-r2')!

    // A is tier 0 (closest to protocol), B is tier 1 (above A)
    expect(agentB.y).toBeLessThan(agentA.y)

    // Tier 0 uses A's height (400), tier 1 uses B's height (500)
    const tier0Y = defaultProtocol.y - TOWER_LAYOUT.TOWER_START_GAP - 400
    expect(agentA.y).toBe(tier0Y)

    const tier1Y = tier0Y - TOWER_LAYOUT.TOWER_GAP - 500
    expect(agentB.y).toBe(tier1Y)
  })
})

// ============================================================================
// computeAllTowerPositions
// ============================================================================

describe('computeAllTowerPositions', () => {
  it('computes positions for multiple protocols', () => {
    const lookups = emptyLookups({
      rosterByStep: {
        'p1': [makeAgent('r1', 'Agent A')],
        'p2': [makeAgent('r2', 'Agent B')],
      },
    })

    const protocolDims = new Map<string, ProtocolDimensions>([
      ['p1', { x: 0, y: 0, width: 560 }],
      ['p2', { x: 700, y: 0, width: 560 }],
    ])

    const result = computeAllTowerPositions(protocolDims, lookups, emptyMeasured)

    expect(result.has('agent-artifact-r1')).toBe(true)
    expect(result.has('agent-artifact-r2')).toBe(true)

    const agent1 = result.get('agent-artifact-r1')!
    const agent2 = result.get('agent-artifact-r2')!

    // Agent2 should be to the right (following its protocol)
    expect(agent2.x).toBeGreaterThan(agent1.x)
  })
})
