import { Collections } from '@/utils/collections'
import type { Point } from '@/utils/geometry'
import type { StepNodeLookups, RosterAgentInfo } from '../mappers/types'
import { AGENT_DEFAULTS } from '../DynamicNode/archetypes'
import { TOWER_LAYOUT } from './autoLayoutConfig'

// ============================================================================
// Tower Entry — agent in a tier
// ============================================================================

type TowerEntry = {
  agentNodeId: string
}

// ============================================================================
// Tiered Tower — agents grouped by dependency depth
// ============================================================================

type TierLayout = {
  tier: number
  entries: TowerEntry[]
}

/**
 * Assign agents to tiers based on depends_on relationships.
 * Tier 0 = root agents (no roster dependencies), tier N = depends on tier N-1.
 */
const computeAgentTiers = (
  roster: readonly RosterAgentInfo[],
): ReadonlyMap<string, number> => {
  const active = roster.filter((a) => a.child_step_id !== null)
  const activeIds = Collections.toSetBy(active, (a) => a.id)
  const tierMap = new Map<string, number>()

  let changed = true
  while (changed) {
    changed = false
    for (const agent of active) {
      if (tierMap.has(agent.id)) continue

      const deps = agent.depends_on.filter((id) => activeIds.has(id))

      if (deps.length === 0) {
        tierMap.set(agent.id, 0)
        changed = true
        continue
      }

      const depTiers: number[] = []
      let allResolved = true
      for (const depId of deps) {
        const t = tierMap.get(depId)
        if (t === undefined) {
          allResolved = false
          break
        }
        depTiers.push(t)
      }

      if (allResolved) {
        tierMap.set(agent.id, Math.max(...depTiers) + 1)
        changed = true
      }
    }
  }

  // Unassigned agents (circular deps or broken refs) → tier 0
  for (const agent of active) {
    if (!tierMap.has(agent.id)) {
      tierMap.set(agent.id, 0)
    }
  }

  return tierMap
}

/**
 * Build tiered tower for a protocol step: agents grouped by dependency tier.
 */
const buildTieredTower = (
  stepId: string,
  lookups: StepNodeLookups,
): TierLayout[] => {
  const roster: readonly RosterAgentInfo[] = lookups.rosterByStep[stepId] ?? []

  const tierMap = computeAgentTiers(roster)

  const tierEntries = new Map<number, TowerEntry[]>()
  for (const agent of roster) {
    if (!agent.child_step_id) continue
    const tier = tierMap.get(agent.id) ?? 0
    const entries = tierEntries.get(tier) ?? []
    entries.push({
      agentNodeId: `agent-artifact-${agent.id}`,
    })
    tierEntries.set(tier, entries)
  }

  const tiers: TierLayout[] = []
  const sortedKeys = Collections.sortedCopy([...tierEntries.keys()], (a, b) => a - b)
  for (const tier of sortedKeys) {
    tiers.push({ tier, entries: tierEntries.get(tier)! })
  }

  return tiers
}

// ============================================================================
// Tower Position Computation
// ============================================================================

type NodeDimensions = {
  width: number
  height: number
}

type ProtocolDimensions = {
  x: number
  y: number
  width: number
}

/**
 * Compute agent positions for a single protocol's tower.
 *
 * Agents are stacked in tiers above the protocol node, centered horizontally
 * on the protocol's current width. Uses actual measured dimensions for each
 * agent node — the tallest agent in a tier determines vertical spacing.
 *
 * Returns a Map of agentNodeId → position for all agents in this protocol's tower.
 */
const computeTowerPositions = (
  stepId: string,
  protocol: ProtocolDimensions,
  lookups: StepNodeLookups,
  measuredDimensions: ReadonlyMap<string, NodeDimensions>,
): ReadonlyMap<string, Point> => {
  const positions = new Map<string, Point>()
  const tiers = buildTieredTower(stepId, lookups)

  if (tiers.length === 0) return positions

  const defaultWidth = AGENT_DEFAULTS.DEFAULT_WIDTH
  const defaultHeight = AGENT_DEFAULTS.DEFAULT_HEIGHT
  const columnCenterX = protocol.x + protocol.width / 2

  let towerCursorY = protocol.y - TOWER_LAYOUT.TOWER_START_GAP

  for (let tierIdx = 0; tierIdx < tiers.length; tierIdx++) {
    const tier = tiers[tierIdx]!

    if (tierIdx > 0) {
      towerCursorY -= TOWER_LAYOUT.TOWER_GAP
    }

    // Find the tallest agent in this tier (determines vertical space needed)
    let tallestHeight = 0
    for (const entry of tier.entries) {
      const dims = measuredDimensions.get(entry.agentNodeId)
      const h = dims?.height ?? defaultHeight
      if (h > tallestHeight) tallestHeight = h
    }

    const agentY = towerCursorY - tallestHeight

    // Compute tier width using actual measured widths
    let tierWidth = 0
    for (let j = 0; j < tier.entries.length; j++) {
      const entry = tier.entries[j]!
      const dims = measuredDimensions.get(entry.agentNodeId)
      tierWidth += dims?.width ?? defaultWidth
      if (j < tier.entries.length - 1) tierWidth += TOWER_LAYOUT.TIER_AGENT_GAP
    }
    const startX = columnCenterX - tierWidth / 2

    // Place each agent, advancing X by its actual width
    let cursorX = startX
    for (let j = 0; j < tier.entries.length; j++) {
      const entry = tier.entries[j]!
      const dims = measuredDimensions.get(entry.agentNodeId)
      const w = dims?.width ?? defaultWidth

      if (entry.agentNodeId) {
        positions.set(entry.agentNodeId, { x: cursorX, y: agentY })
      }
      cursorX += w + TOWER_LAYOUT.TIER_AGENT_GAP
    }

    towerCursorY -= tallestHeight
  }

  return positions
}

/**
 * Compute tower positions for ALL protocol steps.
 *
 * Takes a map of protocol step IDs → current dimensions and measured node
 * dimensions for all agent nodes, then returns positions for all agent nodes
 * across all towers.
 */
const computeAllTowerPositions = (
  protocolDimensions: ReadonlyMap<string, ProtocolDimensions>,
  lookups: StepNodeLookups,
  measuredDimensions: ReadonlyMap<string, NodeDimensions>,
): ReadonlyMap<string, Point> => {
  const allPositions = new Map<string, Point>()

  for (const [stepId, dims] of protocolDimensions) {
    const towerPositions = computeTowerPositions(stepId, dims, lookups, measuredDimensions)
    for (const [nodeId, pos] of towerPositions) {
      allPositions.set(nodeId, pos)
    }
  }

  return allPositions
}

export { computeTowerPositions, computeAllTowerPositions, computeAgentTiers, buildTieredTower }
export type { TowerEntry, TierLayout, ProtocolDimensions, NodeDimensions }
