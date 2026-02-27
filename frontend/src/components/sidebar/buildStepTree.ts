import { Collections } from '@/utils/collections'
import type { WorkflowStep, WorkflowStepEdge, RosterAgent } from '@/types/workflow'
import { toContinuationGutter } from './gutterLines'

// ── Constants ───────────────────────────────────────────────────────────────

/** Execution modes that are pass-through / non-visible in the tree. */
const HIDDEN_MODES: ReadonlySet<string> = new Set(['context', 'input', 'manager'])

// ── Types ───────────────────────────────────────────────────────────────────

type GutterCell =
  | 'blank'
  | 'pipe'
  | 'branch'
  | 'corner'
  | 'fork_start'
  | 'par_mid'
  | 'par_end'
  | 'root_fork'

type StepEntry = {
  readonly kind: 'step'
  readonly step: WorkflowStep
  readonly gutter: readonly GutterCell[]
}

type GapEntry = {
  readonly kind: 'gap'
}

type AgentEntry = {
  readonly kind: 'agent'
  readonly stepId: string
  readonly agentName: string
  readonly gutter: readonly GutterCell[]
}

type TreeEntry = StepEntry | GapEntry | AgentEntry

// ── Graph Helpers ───────────────────────────────────────────────────────────

type Graph = {
  readonly children: Map<string, string[]>
  readonly parents: Map<string, string[]>
  readonly stepMap: Map<string, WorkflowStep>
}

const buildGraph = (
  steps: readonly WorkflowStep[],
  edges: readonly WorkflowStepEdge[],
): Graph => {
  const stepMap = Collections.indexById(steps)
  const children = new Map<string, string[]>()
  const parents = new Map<string, string[]>()

  const n = edges.length
  for (let i = 0; i < n; i++) {
    const e = edges[i]!
    // Only include edges where both endpoints exist
    if (!stepMap.has(e.from_step_id) || !stepMap.has(e.to_step_id)) continue

    const fwd = children.get(e.from_step_id)
    if (fwd) fwd.push(e.to_step_id)
    else children.set(e.from_step_id, [e.to_step_id])

    const rev = parents.get(e.to_step_id)
    if (rev) rev.push(e.from_step_id)
    else parents.set(e.to_step_id, [e.from_step_id])
  }

  return { children, parents, stepMap }
}

// ── Connected Components ────────────────────────────────────────────────────

const findComponents = (
  steps: readonly WorkflowStep[],
  graph: Graph,
): string[][] => {
  const visited = new Set<string>()
  const components: string[][] = []

  const bfs = (startId: string): string[] => {
    const component: string[] = []
    const queue = [startId]
    visited.add(startId)

    while (queue.length > 0) {
      const id = queue.shift()!
      component.push(id)

      const fwd = graph.children.get(id)
      if (fwd) {
        for (let i = 0; i < fwd.length; i++) {
          const next = fwd[i]!
          if (!visited.has(next)) {
            visited.add(next)
            queue.push(next)
          }
        }
      }

      const rev = graph.parents.get(id)
      if (rev) {
        for (let i = 0; i < rev.length; i++) {
          const next = rev[i]!
          if (!visited.has(next)) {
            visited.add(next)
            queue.push(next)
          }
        }
      }
    }

    return component
  }

  const n = steps.length
  for (let i = 0; i < n; i++) {
    const step = steps[i]!
    if (!visited.has(step.id)) {
      components.push(bfs(step.id))
    }
  }

  return components
}

// ── Topological Sort ────────────────────────────────────────────────────────

const topoSort = (
  nodeIds: readonly string[],
  graph: Graph,
): string[] => {
  const nodeSet = Collections.toSet(nodeIds)
  const inDegree = new Map<string, number>()

  for (let i = 0; i < nodeIds.length; i++) {
    inDegree.set(nodeIds[i]!, 0)
  }

  for (let i = 0; i < nodeIds.length; i++) {
    const id = nodeIds[i]!
    const fwd = graph.children.get(id)
    if (!fwd) continue
    for (let j = 0; j < fwd.length; j++) {
      const child = fwd[j]!
      if (nodeSet.has(child)) {
        inDegree.set(child, (inDegree.get(child) ?? 0) + 1)
      }
    }
  }

  const orderOf = (id: string): number => graph.stepMap.get(id)?.display_order ?? 0

  // Seed with zero in-degree nodes
  const queue = Collections.sortedCopy(
    Collections.filterMap(nodeIds, (id) => (inDegree.get(id) === 0 ? id : null)),
    (a, b) => orderOf(a) - orderOf(b),
  )

  const result: string[] = []
  const visited = new Set<string>()

  while (queue.length > 0) {
    const id = queue.shift()!
    if (visited.has(id)) continue
    visited.add(id)
    result.push(id)

    const fwd = graph.children.get(id)
    if (!fwd) continue

    const ready: string[] = []
    for (let j = 0; j < fwd.length; j++) {
      const next = fwd[j]!
      if (!nodeSet.has(next) || visited.has(next)) continue
      const d = (inDegree.get(next) ?? 1) - 1
      inDegree.set(next, d)
      if (d === 0) ready.push(next)
    }

    if (ready.length > 0) {
      ready.sort((a, b) => orderOf(a) - orderOf(b))
      for (let j = 0; j < ready.length; j++) {
        queue.push(ready[j]!)
      }
    }
  }

  // Append cycle remnants
  if (visited.size < nodeIds.length) {
    for (let i = 0; i < nodeIds.length; i++) {
      const id = nodeIds[i]!
      if (!visited.has(id)) result.push(id)
    }
  }

  return result
}

// ── Merge Point Detection ───────────────────────────────────────────────────

/**
 * For each fork point (outDegree > 1), find the merge node where all
 * branches reconverge. Uses reachability set intersection — the merge
 * is the first node in topo order reachable from ALL branches.
 */
const computeMergePoints = (
  topoOrder: readonly string[],
  graph: Graph,
  scope: ReadonlySet<string>,
): Map<string, string | null> => {
  const mergeOf = new Map<string, string | null>()

  const reachableFrom = (startId: string): Set<string> => {
    const reached = new Set<string>()
    const queue = [startId]
    while (queue.length > 0) {
      const id = queue.shift()!
      if (reached.has(id)) continue
      reached.add(id)
      const fwd = graph.children.get(id)
      if (!fwd) continue
      for (let j = 0; j < fwd.length; j++) {
        const next = fwd[j]!
        if (scope.has(next)) queue.push(next)
      }
    }
    return reached
  }

  const n = topoOrder.length
  for (let i = 0; i < n; i++) {
    const id = topoOrder[i]!
    const fwd = graph.children.get(id)
    if (!fwd) continue

    // Only consider children within scope
    const scopedChildren: string[] = []
    for (let j = 0; j < fwd.length; j++) {
      if (scope.has(fwd[j]!)) scopedChildren.push(fwd[j]!)
    }
    if (scopedChildren.length < 2) continue

    // Compute reachable sets per branch
    const reachableSets: Set<string>[] = []
    for (let j = 0; j < scopedChildren.length; j++) {
      reachableSets.push(reachableFrom(scopedChildren[j]!))
    }

    // Merge = first node in topo order in ALL reachable sets
    let merge: string | null = null
    for (let k = 0; k < n; k++) {
      const candidate = topoOrder[k]!
      if (candidate === id) continue
      let inAll = true
      for (let j = 0; j < reachableSets.length; j++) {
        if (!reachableSets[j]!.has(candidate)) {
          inAll = false
          break
        }
      }
      if (inAll) {
        merge = candidate
        break
      }
    }

    mergeOf.set(id, merge)
  }

  return mergeOf
}

// ── Reachability (for branch sub-DAG collection) ────────────────────────────

/**
 * Collect all nodes reachable from `start` within `scope`, stopping
 * before `stopBefore`. Returns the set including `start`.
 */
const reachableBefore = (
  start: string,
  stopBefore: string | null,
  graph: Graph,
  scope: ReadonlySet<string>,
): Set<string> => {
  const reached = new Set<string>()
  const queue = [start]
  while (queue.length > 0) {
    const id = queue.shift()!
    if (reached.has(id)) continue
    if (id === stopBefore) continue
    if (!scope.has(id)) continue
    reached.add(id)
    const fwd = graph.children.get(id)
    if (!fwd) continue
    for (let j = 0; j < fwd.length; j++) {
      queue.push(fwd[j]!)
    }
  }
  return reached
}

// ── Linearization ───────────────────────────────────────────────────────────

const linearizeComponent = (
  componentIds: readonly string[],
  graph: Graph,
): StepEntry[] => {
  const scope = Collections.toSet(componentIds)
  const order = topoSort(componentIds, graph)
  const mergeOf = computeMergePoints(order, graph, scope)

  const result: StepEntry[] = []
  const emitted = new Set<string>()

  const orderOf = (id: string): number => graph.stepMap.get(id)?.display_order ?? 0

  // Find roots within the component
  const roots: string[] = []
  for (let i = 0; i < componentIds.length; i++) {
    const id = componentIds[i]!
    const pars = graph.parents.get(id)
    const hasParentInScope = pars ? pars.some((p) => scope.has(p)) : false
    if (!hasParentInScope) roots.push(id)
  }
  roots.sort((a, b) => orderOf(a) - orderOf(b))

  // Check for fan-in from independent roots (multiple roots that converge)
  const isFanInRoots = roots.length > 1

  const emit = (id: string, gutter: readonly GutterCell[]) => {
    if (emitted.has(id)) return
    emitted.add(id)
    const step = graph.stepMap.get(id)
    if (!step) return
    result.push({ kind: 'step', step, gutter })
  }

  /**
   * Emit a node and handle its fork (if it is a fork point) recursively.
   * Returns the merge node ID if a fork was processed, so the caller
   * can continue from the merge point.
   */
  const emitNodeWithFork = (
    id: string,
    gutter: readonly GutterCell[],
    prefix: readonly GutterCell[],
    stopBefore: string | null,
    isLastInSegment: boolean,
  ): string | null => {
    const mergeNode = mergeOf.get(id) ?? null
    const fwd = graph.children.get(id)
    const scopedChildren: string[] = []
    if (fwd) {
      for (let j = 0; j < fwd.length; j++) {
        const child = fwd[j]!
        if (scope.has(child) && child !== stopBefore && !emitted.has(child)) {
          scopedChildren.push(child)
        }
      }
      scopedChildren.sort((a, b) => orderOf(a) - orderOf(b))
    }

    const isFork = scopedChildren.length > 1

    if (!isFork) {
      emit(id, gutter)
      return null
    }

    // Emit this node as a sequential step (fork point)
    emit(id, gutter)

    // Process the fork's parallel branches
    processFork(scopedChildren, mergeNode, prefix, stopBefore, isLastInSegment)

    return mergeNode
  }

  /**
   * Process a fork: emit parallel branches and recurse into sub-DAGs.
   */
  const processFork = (
    branches: readonly string[],
    mergeNode: string | null,
    parentPrefix: readonly GutterCell[],
    stopBefore: string | null,
    _isLastSegment: boolean,
  ) => {
    for (let b = 0; b < branches.length; b++) {
      const branchStart = branches[b]!
      const isFirstBranch = b === 0
      const isLastBranch = b === branches.length - 1

      const branchCell: GutterCell = isFirstBranch
        ? 'fork_start'
        : isLastBranch
          ? 'par_end'
          : 'par_mid'

      // Collect the sub-DAG for this branch (before the merge point)
      const branchNodes = reachableBefore(branchStart, mergeNode, graph, scope)

      // Gutter for the branch start — all branches use the same column
      // structure so parallel siblings align at equal visual depth:
      // [...parentPrefix, pipe, fork_start/par_mid/par_end]
      const branchGutter: GutterCell[] = [...parentPrefix, 'pipe', branchCell]

      // The prefix for content INSIDE this branch:
      // [...parentPrefix, pipe, pipe] — pipe for fork group + pipe for branch depth
      const innerPrefix: GutterCell[] = [...parentPrefix, 'pipe', 'pipe']

      // Check if the branch start is itself a fork point
      const branchMerge = mergeOf.get(branchStart) ?? null
      const branchFwd = graph.children.get(branchStart)
      const branchScopedChildren: string[] = []
      if (branchFwd) {
        for (let j = 0; j < branchFwd.length; j++) {
          const child = branchFwd[j]!
          if (scope.has(child) && child !== mergeNode && child !== stopBefore && !emitted.has(child)) {
            branchScopedChildren.push(child)
          }
        }
        branchScopedChildren.sort((a, b) => orderOf(a) - orderOf(b))
      }

      const branchIsFork = branchScopedChildren.length > 1

      emit(branchStart, branchGutter)
      branchNodes.delete(branchStart)

      if (branchIsFork) {
        // Branch start is itself a fork — process its fork inline.
        // The nested fork prefix = innerPrefix + pipe for B's content column.
        // This is the prefix at which D/E live (before adding their own fork columns).
        const nestedPrefix: GutterCell[] = [...innerPrefix, 'pipe']
        processFork(branchScopedChildren, branchMerge, nestedPrefix, mergeNode, isLastBranch)

        // Continue from the branch's merge point to the outer merge.
        // The merge node (F) sits at nestedPrefix depth — same level as the
        // fork's content, now that the fork group has closed.
        if (branchMerge && !emitted.has(branchMerge)) {
          const afterBranchMerge: string[] = []
          for (const nodeId of branchNodes) {
            if (!emitted.has(nodeId)) afterBranchMerge.push(nodeId)
          }
          if (branchNodes.has(branchMerge) || scope.has(branchMerge)) {
            afterBranchMerge.push(branchMerge)
          }
          if (afterBranchMerge.length > 0) {
            // isLastSegment=true: F is the last content in B's branch,
            // so the final node should get 'corner' not 'branch'.
            linearize(afterBranchMerge, mergeNode, nestedPrefix, true)
          }
        }
      } else if (branchNodes.size > 0) {
        // Recursively linearize the branch's remaining sub-DAG
        linearize([...branchNodes], mergeNode, innerPrefix, isLastBranch)
      }
    }
  }

  /**
   * Linearize a segment of the DAG. Processes nodes in topo order,
   * handling fork/merge recursively.
   */
  const linearize = (
    nodeIds: readonly string[],
    stopBefore: string | null,
    prefix: readonly GutterCell[],
    isLastSegment: boolean,
  ) => {
    // Filter to topo-order, only nodes in our set that haven't been emitted
    const nodeSet = Collections.toSet(nodeIds)
    const ordered: string[] = []
    for (let i = 0; i < order.length; i++) {
      const id = order[i]!
      if (id === stopBefore) continue
      if (emitted.has(id)) continue
      if (nodeSet.has(id)) ordered.push(id)
    }

    let idx = 0
    while (idx < ordered.length) {
      const id = ordered[idx]!
      if (emitted.has(id)) { idx++; continue }

      // Determine if more nodes follow in this segment
      const remaining = countRemaining(ordered, idx + 1, emitted, null, stopBefore)
      const isLast = isLastSegment && remaining === 0
      const cell: GutterCell = isLast ? 'corner' : 'branch'

      const mergeNode = emitNodeWithFork(id, [...prefix, cell], prefix, stopBefore, isLastSegment)

      if (mergeNode !== null) {
        // Fork was processed — continue from the merge node
        if (!emitted.has(mergeNode)) {
          const mergeOnward: string[] = []
          for (let k = idx + 1; k < ordered.length; k++) {
            if (!emitted.has(ordered[k]!)) mergeOnward.push(ordered[k]!)
          }
          // Add merge node and anything reachable from it
          mergeOnward.push(mergeNode)
          const reachFromMerge = reachableBefore(mergeNode, stopBefore, graph, scope)
          for (const rid of reachFromMerge) {
            if (!emitted.has(rid) && !mergeOnward.includes(rid)) {
              mergeOnward.push(rid)
            }
          }
          linearize(mergeOnward, stopBefore, prefix, isLastSegment)
        }
        return
      }

      idx++
    }
  }

  if (isFanInRoots) {
    // Multiple roots that converge — use root_fork pattern
    // Find the convergence point
    const rootReachable = roots.map((r) => reachableBefore(r, null, graph, scope))

    // Find first node in topo order that ALL roots reach
    let convergence: string | null = null
    for (let i = 0; i < order.length; i++) {
      const candidate = order[i]!
      if (roots.includes(candidate)) continue
      let inAll = true
      for (let j = 0; j < rootReachable.length; j++) {
        if (!rootReachable[j]!.has(candidate)) {
          inAll = false
          break
        }
      }
      if (inAll) {
        convergence = candidate
        break
      }
    }

    if (convergence !== null) {
      // Emit roots as parallel siblings using root_fork pattern
      for (let i = 0; i < roots.length; i++) {
        const rootId = roots[i]!
        const isFirst = i === 0
        const isLastRoot = i === roots.length - 1

        const cell: GutterCell = isFirst ? 'root_fork' : isLastRoot ? 'par_end' : 'par_mid'
        emit(rootId, [cell])

        // Collect sub-DAG for this root branch before convergence
        const branchNodes = reachableBefore(rootId, convergence, graph, scope)
        branchNodes.delete(rootId)

        if (branchNodes.size > 0) {
          linearize([...branchNodes], convergence, ['pipe'], isLastRoot)
        }
      }

      // Continue from convergence
      const remaining: string[] = []
      for (let i = 0; i < order.length; i++) {
        const id = order[i]!
        if (!emitted.has(id)) remaining.push(id)
      }
      linearize(remaining, null, [], true)
    } else {
      // No convergence — treat as independent chains within the component
      // (shouldn't happen since they're in the same component, but be safe)
      linearize(order, null, [], true)
    }
  } else {
    // Single root or no roots — straightforward linearization
    linearize(order, null, [], true)
  }

  // Emit any remaining nodes that weren't reached (orphans within component)
  for (let i = 0; i < componentIds.length; i++) {
    const id = componentIds[i]!
    if (!emitted.has(id)) {
      emit(id, ['corner'])
    }
  }

  return result
}

/** Count remaining non-emitted nodes after index `startIdx` in `ordered`, excluding merge/stop */
const countRemaining = (
  ordered: readonly string[],
  startIdx: number,
  emitted: ReadonlySet<string>,
  mergeNode: string | null,
  stopBefore: string | null,
): number => {
  let count = 0
  for (let i = startIdx; i < ordered.length; i++) {
    const id = ordered[i]!
    if (id === mergeNode) continue
    if (id === stopBefore) continue
    if (emitted.has(id)) continue
    count++
  }
  return count
}

// ── Main Entry Point ────────────────────────────────────────────────────────

/**
 * Build a flat list of TreeEntry objects with DAG-aware gutter metadata.
 *
 * Each entry carries a `gutter: GutterCell[]` array that encodes the
 * box-drawing characters for that row. The gutter replaces the old
 * `depth` + `isLast` approach and can represent forks, merges, parallel
 * branches, nested forks, and independent chains.
 *
 * `rosterByStep` provides agent rosters for workforce steps — each agent
 * becomes a child row beneath its parent step in the tree.
 */
const buildStepTree = (
  steps: readonly WorkflowStep[],
  edges: readonly WorkflowStepEdge[],
  rosterByStep: Readonly<Record<string, RosterAgent[]>>,
): TreeEntry[] => {
  // Filter out non-visible steps (context, input) — consistent with topoSortStepIds
  const visible = Collections.filterMap(steps, (s) =>
    HIDDEN_MODES.has(s.execution_mode) ? null : s,
  )
  if (visible.length === 0) return []

  const graph = buildGraph(visible, edges)
  const components = findComponents(visible, graph)

  if (components.length === 0) return []

  // Sort components by the minimum display_order of their roots
  const orderOf = (id: string): number => graph.stepMap.get(id)?.display_order ?? 0
  components.sort((a, b) => {
    const minA = Math.min(...a.map(orderOf))
    const minB = Math.min(...b.map(orderOf))
    return minA - minB
  })

  const stepEntries: TreeEntry[] = []

  for (let c = 0; c < components.length; c++) {
    if (c > 0) stepEntries.push({ kind: 'gap' })
    const entries = linearizeComponent(components[c]!, graph)
    for (let i = 0; i < entries.length; i++) {
      stepEntries.push(entries[i]!)
    }
  }

  // Post-process: insert agent rows after workforce steps
  const result: TreeEntry[] = []
  for (let i = 0; i < stepEntries.length; i++) {
    const entry = stepEntries[i]!
    result.push(entry)

    if (entry.kind !== 'step' || entry.step.execution_mode !== 'workforce') continue

    const agents = rosterByStep[entry.step.id]
    if (!agents || agents.length === 0) continue

    const continuation = toContinuationGutter(entry.gutter)
    const sorted = Collections.sortedCopy(agents, (a, b) => a.execution_order - b.execution_order)

    for (let j = 0; j < sorted.length; j++) {
      const agent = sorted[j]!
      const isLast = j === sorted.length - 1
      const agentGutter: GutterCell[] = [...continuation, isLast ? 'corner' : 'branch']
      result.push({
        kind: 'agent',
        stepId: entry.step.id,
        agentName: agent.name,
        gutter: agentGutter,
      })
    }
  }

  return result
}

export { buildStepTree }
export type { TreeEntry, StepEntry, GapEntry, AgentEntry, GutterCell }
