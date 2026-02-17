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

/**
 * Build tower entries for a protocol step: each roster agent paired
 * with its assigned document (if any).
 */
const buildTower = (
  stepId: string,
  lookups: StepNodeLookups,
): TowerEntry[] => {
  const roster: readonly RosterAgentInfo[] = lookups.rosterByStep[stepId] ?? []
  const defs: readonly DocumentDefInfo[] = lookups.documentDefsByStep[stepId] ?? []

  // Build doc lookup by agent_roster_entry_id
  const docByRosterId = new Map<string, string>()
  for (const def of defs) {
    if (def.agent_roster_entry_id) {
      docByRosterId.set(def.agent_roster_entry_id, def.id)
    }
  }

  const entries: TowerEntry[] = []
  for (const agent of roster) {
    if (!agent.child_step_id) continue
    entries.push({
      agentNodeId: `agent-artifact-${agent.id}`,
      documentNodeId: docByRosterId.has(agent.id) ? `doc-artifact-${docByRosterId.get(agent.id)!}` : null,
    })
  }

  // Also add unassigned documents (not linked to any agent)
  for (const def of defs) {
    if (!def.agent_roster_entry_id) {
      entries.push({
        agentNodeId: '', // no agent — just a floating document
        documentNodeId: `doc-artifact-${def.id}`,
      })
    }
  }

  return entries
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
  for (const id of allSpineIds) {
    if (!spineOrder.includes(id)) spineOrder.push(id)
  }

  // 3. Build towers for each protocol
  const towersByStep = new Map<string, TowerEntry[]>()
  for (const protocol of protocols) {
    towersByStep.set(protocol.id, buildTower(protocol.id, lookups))
  }

  // 4. Compute node dimensions
  const protocolWidth = NODE_DIMENSIONS[CanvasNodeKind.PROTOCOL].defaultWidth
  const protocolHeight = NODE_DIMENSIONS[CanvasNodeKind.PROTOCOL].defaultHeight
  const inputWidth = NODE_DIMENSIONS[CanvasNodeKind.INPUT].defaultWidth
  const inputHeight = NODE_DIMENSIONS[CanvasNodeKind.INPUT].defaultHeight
  const contextWidth = NODE_DIMENSIONS[CanvasNodeKind.CONTEXT].defaultWidth
  const agentWidth = AGENT_DEFAULTS.DEFAULT_WIDTH
  const agentHeight = AGENT_DEFAULTS.DEFAULT_HEIGHT
  const docWidth = DOCUMENT_NODE.DEFAULT_WIDTH
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
    const tower = towersByStep.get(stepId) ?? []

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

    // Column width: max of node width and tower width (agent + gap + doc)
    const towerWidth = tower.length > 0
      ? agentWidth + AUTO_LAYOUT.DOC_GAP + docWidth
      : 0
    const columnWidth = Math.max(nodeWidth, towerWidth)

    // Center the node in its column
    const nodeX = cursorX + (columnWidth - nodeWidth) / 2

    // Place spine node
    positions.set(stepId, { x: nodeX, y: spineY })

    // 6. Stack tower entries upward from spine
    if (tower.length > 0) {
      const towerStartX = cursorX + (columnWidth - towerWidth) / 2

      for (let i = 0; i < tower.length; i++) {
        const entry = tower[i]!
        const entryY = spineY - AUTO_LAYOUT.TOWER_START_GAP - nodeHeight
          - i * (agentHeight + AUTO_LAYOUT.TOWER_GAP)

        // Place agent node
        if (entry.agentNodeId) {
          positions.set(entry.agentNodeId, { x: towerStartX, y: entryY })
        }

        // Place document node to the right of agent
        if (entry.documentNodeId) {
          const docX = entry.agentNodeId
            ? towerStartX + agentWidth + AUTO_LAYOUT.DOC_GAP
            : towerStartX
          positions.set(entry.documentNodeId, { x: docX, y: entryY })
        }
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

export { computeAutoLayout, classifyStep, topologicalSort, buildTower }
export type { NodeRole, TowerEntry }
