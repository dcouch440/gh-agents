import type { WorkflowStep, WorkflowStepEdge, RosterAgent } from '@/types/workflow'

// ── Types ───────────────────────────────────────────────────────────────────

type StepEntry = {
  readonly kind: 'step'
  readonly step: WorkflowStep
  readonly depth: number
  readonly isLast: boolean
}

type AgentEntry = {
  readonly kind: 'agent'
  readonly agent: RosterAgent
  readonly stepId: string
  readonly depth: number
  readonly isLast: boolean
}

type TreeEntry = StepEntry | AgentEntry

// ── Constants ───────────────────────────────────────────────────────────────

const MANAGER_NAMES = new Set(['manager', 'designer'])

const isManager = (agent: RosterAgent): boolean =>
  MANAGER_NAMES.has(agent.name.toLowerCase())

// ── Tree Derivation ─────────────────────────────────────────────────────────

/**
 * Build a flat list of TreeEntry objects from steps, edges, and roster agents.
 *
 * Hierarchy: steps form the DAG tree (edges determine parent-child).
 * Roster agents appear as children under their parent step, with the
 * manager agent filtered out.
 */
const buildStepTree = (
  steps: readonly WorkflowStep[],
  edges: readonly WorkflowStepEdge[],
  rosterByStep: Readonly<Record<string, RosterAgent[]>>,
): TreeEntry[] => {
  if (steps.length === 0) return []

  // adjacency: parent → children
  const children = new Map<string, string[]>()
  const incomingCount = new Map<string, number>()

  for (const edge of edges) {
    const list = children.get(edge.from_step_id)
    if (list) {
      list.push(edge.to_step_id)
    } else {
      children.set(edge.from_step_id, [edge.to_step_id])
    }
    incomingCount.set(edge.to_step_id, (incomingCount.get(edge.to_step_id) ?? 0) + 1)
  }

  const stepMap = new Map<string, WorkflowStep>()
  for (const step of steps) {
    stepMap.set(step.id, step)
  }

  // Roots = steps with no incoming edges
  const roots = steps
    .filter((s) => !incomingCount.has(s.id) || incomingCount.get(s.id) === 0)
    .sort((a, b) => a.display_order - b.display_order)

  // DFS to build flat list
  const result: TreeEntry[] = []
  const visited = new Set<string>()

  const emitAgents = (stepId: string, depth: number) => {
    const roster = rosterByStep[stepId]
    if (!roster) return
    const agents = roster.filter((a) => !isManager(a)).sort((a, b) => a.execution_order - b.execution_order)
    for (let i = 0; i < agents.length; i++) {
      result.push({
        kind: 'agent',
        agent: agents[i]!,
        stepId,
        depth: depth + 1,
        isLast: i === agents.length - 1,
      })
    }
  }

  const walk = (stepId: string, depth: number, isLast: boolean) => {
    if (visited.has(stepId)) return
    visited.add(stepId)

    const step = stepMap.get(stepId)
    if (!step) return

    result.push({ kind: 'step', step, depth, isLast })
    emitAgents(stepId, depth)

    const childIds = children.get(stepId)
    if (!childIds) return

    const sorted = childIds
      .filter((id) => !visited.has(id))
      .map((id) => stepMap.get(id))
      .filter((s): s is WorkflowStep => s !== undefined)
      .sort((a, b) => a.display_order - b.display_order)

    for (let i = 0; i < sorted.length; i++) {
      walk(sorted[i]!.id, depth + 1, i === sorted.length - 1)
    }
  }

  for (let i = 0; i < roots.length; i++) {
    walk(roots[i]!.id, 0, i === roots.length - 1)
  }

  // Orphaned steps
  for (const step of steps) {
    if (!visited.has(step.id)) {
      result.push({ kind: 'step', step, depth: 0, isLast: true })
      emitAgents(step.id, 0)
    }
  }

  return result
}

export { buildStepTree }
export type { TreeEntry, StepEntry, AgentEntry }
