import { Collections } from '@/utils/collections'
import type { Point } from '@/utils/geometry'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import type { StepNodeLookups, RosterAgentInfo, DocumentDefInfo } from '../mappers/types'
import { NODE_DIMENSIONS } from '../nodeDimensions'
import { CanvasNodeKind } from '../canvasKinds'
import { AGENT_DEFAULTS } from '../DynamicNode/archetypes'
import { DOCUMENT_NODE } from '../DocumentNode'
import { AUTO_LAYOUT } from './autoLayoutConfig'

// ============================================================================
// Node Classification
// ============================================================================

type NodeRole = 'input' | 'context' | 'protocol' | 'step'

const classifyStep = (step: WorkflowStep, protocolsByStep: ReadonlyMap<string, unknown>): NodeRole => {
  if (step.execution_mode === 'input') return 'input'
  if (step.execution_mode === 'context') return 'context'
  if (step.execution_mode === 'workforce' || step.execution_mode === 'room') return 'protocol'
  if (protocolsByStep.has(step.id)) return 'protocol'
  return 'step'
}

// ============================================================================
// Topological Sort — spine ordering
// ============================================================================

/**
 * Kahn's algorithm for topological sort, restricted to the given step IDs.
 * Returns step IDs in execution order (left-to-right for spine).
 */
const topologicalSort = (
  stepIds: ReadonlySet<string>,
  edges: readonly WorkflowStepEdge[],
): string[] => {
  const inDegree = new Map<string, number>()
  const adjacency = new Map<string, string[]>()

  for (const id of stepIds) {
    inDegree.set(id, 0)
    adjacency.set(id, [])
  }

  for (const edge of edges) {
    if (!stepIds.has(edge.from_step_id) || !stepIds.has(edge.to_step_id)) continue
    adjacency.get(edge.from_step_id)!.push(edge.to_step_id)
    inDegree.set(edge.to_step_id, (inDegree.get(edge.to_step_id) ?? 0) + 1)
  }

  const queue: string[] = []
  for (const [id, deg] of inDegree) {
    if (deg === 0) queue.push(id)
  }

  const sorted: string[] = []
  while (queue.length > 0) {
    const current = queue.shift()!
    sorted.push(current)
    for (const neighbor of adjacency.get(current) ?? []) {
      const newDeg = (inDegree.get(neighbor) ?? 1) - 1
      inDegree.set(neighbor, newDeg)
      if (newDeg === 0) queue.push(neighbor)
    }
  }

  return sorted
}

// ============================================================================
// Tower Entry — agent+document pair
// ============================================================================

type TowerEntry = {
  agentNodeId: string
  documentNodeId: string | null
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
 * Returns roster agents grouped by tier, ordered ascending.
 */
const computeAgentTiers = (
  roster: readonly RosterAgentInfo[],
): ReadonlyMap<string, number> => {
  // Only consider agents with child_step_id (active on canvas)
  const active = roster.filter((a) => a.child_step_id !== null)
  const activeIds = Collections.toSetBy(active, (a) => a.id)
  const tierMap = new Map<string, number>()

  // Iteratively assign tiers until all agents are placed
  let changed = true
  while (changed) {
    changed = false
    for (const agent of active) {
      if (tierMap.has(agent.id)) continue

      // Filter depends_on to only reference active roster agents
      const deps = agent.depends_on.filter((id) => activeIds.has(id))

      if (deps.length === 0) {
        tierMap.set(agent.id, 0)
        changed = true
        continue
      }

      // All dependencies must be assigned before we can compute this agent's tier
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

  // Any remaining unassigned agents (circular deps or broken refs) → tier 0
  for (const agent of active) {
    if (!tierMap.has(agent.id)) {
      tierMap.set(agent.id, 0)
    }
  }

  return tierMap
}

/**
 * Build tiered tower for a protocol step: agents grouped by dependency tier,
 * each paired with its assigned document (if any).
 */
const buildTieredTower = (
  stepId: string,
  lookups: StepNodeLookups,
): TierLayout[] => {
  const roster: readonly RosterAgentInfo[] = lookups.rosterByStep[stepId] ?? []
  const defs: readonly DocumentDefInfo[] = lookups.documentDefsByStep[stepId] ?? []

  // Build doc lookup by agent_roster_entry_id
  const docByRosterId = new Map<string, string>()
  for (const def of defs) {
    if (def.agent_roster_entry_id) {
      docByRosterId.set(def.agent_roster_entry_id, def.id)
    }
  }

  // Compute tier assignments
  const tierMap = computeAgentTiers(roster)

  // Group entries by tier
  const tierEntries = new Map<number, TowerEntry[]>()
  for (const agent of roster) {
    if (!agent.child_step_id) continue
    const tier = tierMap.get(agent.id) ?? 0
    const entries = tierEntries.get(tier) ?? []
    entries.push({
      agentNodeId: `agent-artifact-${agent.id}`,
      documentNodeId: docByRosterId.has(agent.id) ? `doc-artifact-${docByRosterId.get(agent.id)!}` : null,
    })
    tierEntries.set(tier, entries)
  }

  // Add unassigned documents as tier 0 entries
  for (const def of defs) {
    if (!def.agent_roster_entry_id) {
      const entries = tierEntries.get(0) ?? []
      entries.push({
        agentNodeId: '',
        documentNodeId: `doc-artifact-${def.id}`,
      })
      tierEntries.set(0, entries)
    }
  }

  // Sort tiers ascending (tier 0 closest to protocol)
  const tiers: TierLayout[] = []
  const sortedKeys = Collections.sortedCopy([...tierEntries.keys()], (a, b) => a - b)
  for (const tier of sortedKeys) {
    tiers.push({ tier, entries: tierEntries.get(tier)! })
  }

  return tiers
}

// ============================================================================
// Main Layout Algorithm
// ============================================================================

/**
 * Compute auto-layout positions for all canvas nodes using the spine+tower model.
 *
 * The spine runs left-to-right: Input → Context → Protocol(s) → remaining steps.
 * Each protocol grows a vertical tower of agent+doc pairs above it.
 * Notes hang below each protocol.
 *
 * Returns a Map of nodeId → position for ALL nodes (step nodes + virtual artifact nodes).
 */
const computeAutoLayout = (
  steps: readonly WorkflowStep[],
  edges: readonly WorkflowStepEdge[],
  lookups: StepNodeLookups,
): ReadonlyMap<string, Point> => {
  const positions = new Map<string, Point>()

  if (steps.length === 0) return positions

  // 1. Classify all steps
  const inputs: WorkflowStep[] = []
  const contexts: WorkflowStep[] = []
  const protocols: WorkflowStep[] = []
  const regularSteps: WorkflowStep[] = []

  for (const step of steps) {
    const role = classifyStep(step, lookups.protocolsByStep)
    switch (role) {
      case 'input': inputs.push(step); break
      case 'context': contexts.push(step); break
      case 'protocol': protocols.push(step); break
      case 'step': regularSteps.push(step); break
    }
  }

  // 2. Build spine order via topological sort
  const allSpineIds = new Set<string>()
  for (const s of inputs) allSpineIds.add(s.id)
  for (const s of contexts) allSpineIds.add(s.id)
  for (const s of protocols) allSpineIds.add(s.id)
  for (const s of regularSteps) allSpineIds.add(s.id)

  const spineOrder = topologicalSort(allSpineIds, edges)

  // If topo sort missed some (disconnected nodes), append them
  const spineOrderSet = Collections.toSet(spineOrder)
  for (const id of allSpineIds) {
    if (!spineOrderSet.has(id)) spineOrder.push(id)
  }

  // 3. Build tiered towers for each protocol
  const towersByStep = new Map<string, TierLayout[]>()
  for (const protocol of protocols) {
    towersByStep.set(protocol.id, buildTieredTower(protocol.id, lookups))
  }

  // 4. Compute node dimensions
  const protocolWidth = NODE_DIMENSIONS[CanvasNodeKind.PROTOCOL].defaultWidth
  const protocolHeight = NODE_DIMENSIONS[CanvasNodeKind.PROTOCOL].defaultHeight
  const inputWidth = NODE_DIMENSIONS[CanvasNodeKind.INPUT].defaultWidth
  const inputHeight = NODE_DIMENSIONS[CanvasNodeKind.INPUT].defaultHeight
  const contextWidth = NODE_DIMENSIONS[CanvasNodeKind.CONTEXT].defaultWidth
  const agentWidth = AGENT_DEFAULTS.DEFAULT_WIDTH
  const agentHeight = AGENT_DEFAULTS.DEFAULT_HEIGHT
  const docHeight = DOCUMENT_NODE.DEFAULT_HEIGHT
  const stepWidth = NODE_DIMENSIONS[CanvasNodeKind.STEP].defaultWidth

  const spineY = AUTO_LAYOUT.SPINE_Y

  // 5. Lay out spine left-to-right
  let cursorX = 0
  const stepById = new Map<string, WorkflowStep>()
  for (const step of steps) stepById.set(step.id, step)

  for (const stepId of spineOrder) {
    const step = stepById.get(stepId)
    if (!step) continue

    const role = classifyStep(step, lookups.protocolsByStep)
    const tiers = towersByStep.get(stepId) ?? []

    // Determine this node's width and its column width
    let nodeWidth: number
    let nodeHeight: number

    switch (role) {
      case 'input':
        nodeWidth = inputWidth
        nodeHeight = inputHeight
        break
      case 'context':
        nodeWidth = contextWidth
        nodeHeight = inputHeight
        break
      case 'protocol':
        nodeWidth = protocolWidth
        nodeHeight = protocolHeight
        break
      case 'step':
        nodeWidth = stepWidth
        nodeHeight = NODE_DIMENSIONS[CanvasNodeKind.STEP].defaultHeight
        break
    }

    // Column width: max of node width and widest tier (agents side-by-side)
    let widestTierWidth = 0
    for (const tier of tiers) {
      const tierWidth = tier.entries.length * agentWidth + (tier.entries.length - 1) * AUTO_LAYOUT.TIER_AGENT_GAP
      if (tierWidth > widestTierWidth) widestTierWidth = tierWidth
    }
    const columnWidth = Math.max(nodeWidth, widestTierWidth)

    // Center the node in its column
    const nodeX = cursorX + (columnWidth - nodeWidth) / 2
    const columnCenterX = cursorX + columnWidth / 2

    // Place spine node
    positions.set(stepId, { x: nodeX, y: spineY })

    // 6. Place tiered tower above protocol (docs above agents)
    if (tiers.length > 0) {
      // Cumulative cursor tracks the bottom edge of the next available slot,
      // moving upward (negative Y) as tiers stack above the protocol.
      let towerCursorY = spineY - AUTO_LAYOUT.TOWER_START_GAP

      for (let tierIdx = 0; tierIdx < tiers.length; tierIdx++) {
        const tier = tiers[tierIdx]!
        const thisHasDoc = tier.entries.some((e) => e.documentNodeId !== null)
        const slotHeight = agentHeight + (thisHasDoc ? AUTO_LAYOUT.DOC_GAP + docHeight : 0)

        if (tierIdx > 0) {
          towerCursorY -= AUTO_LAYOUT.TOWER_GAP
        }

        // Agent top = cursor bottom minus agent height
        const agentY = towerCursorY - agentHeight

        // Spread entries horizontally, centered on column
        const tierWidth = tier.entries.length * agentWidth + (tier.entries.length - 1) * AUTO_LAYOUT.TIER_AGENT_GAP
        const startX = columnCenterX - tierWidth / 2

        for (let j = 0; j < tier.entries.length; j++) {
          const entry = tier.entries[j]!
          const entryX = startX + j * (agentWidth + AUTO_LAYOUT.TIER_AGENT_GAP)

          // Place agent node
          if (entry.agentNodeId) {
            positions.set(entry.agentNodeId, { x: entryX, y: agentY })
          }

          // Place document node above agent
          if (entry.documentNodeId) {
            positions.set(entry.documentNodeId, { x: entryX, y: agentY - docHeight - AUTO_LAYOUT.DOC_GAP })
          }
        }

        // Advance cursor upward past this tier's full slot
        towerCursorY -= slotHeight
      }
    }

    // 7. Place notes below protocol
    const notesContent = lookups.notesByStep[stepId]
    if (notesContent) {
      const notesNodeId = `notes-${stepId}`
      positions.set(notesNodeId, {
        x: nodeX,
        y: spineY + nodeHeight + AUTO_LAYOUT.NOTES_GAP,
      })
    }

    // Advance cursor
    cursorX += columnWidth + AUTO_LAYOUT.SPINE_GAP
  }

  return positions
}

export { computeAutoLayout, classifyStep, topologicalSort, buildTieredTower, computeAgentTiers }
export type { NodeRole, TowerEntry, TierLayout }
