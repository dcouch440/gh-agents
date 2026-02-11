import type { WorkflowStep } from '@/types/workflow'
import { PROTOCOL_TYPE_COLORS, GREYSCALE_ACCENT } from '../constants'
import type { ProtocolStepInfo, ProtocolGroupEntry } from './types'

const isDocumenterStep = (
  step: { id: string; execution_mode: string },
  protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>,
): boolean =>
  step.execution_mode === 'documenter' || protocolsByStep.get(step.id)?.protocol_type === 'documenter'

/**
 * BFS from each protocol step to find all connected non-protocol nodes.
 * Returns a map: stepId -> { protocolColor, protocolStepId }
 */
const computeProtocolGroups = (
  steps: WorkflowStep[],
  edges: ReadonlyArray<{ from_step_id: string; to_step_id: string }>,
  protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>,
): ReadonlyMap<string, ProtocolGroupEntry> => {
  // Identify all protocol step IDs (from stepProtocols map or execution_mode)
  const protocolStepIds = new Set<string>()
  for (const step of steps) {
    if (protocolsByStep.has(step.id) || isDocumenterStep(step, protocolsByStep)) {
      protocolStepIds.add(step.id)
    }
  }

  // Build bidirectional adjacency list
  const adjacency = new Map<string, Set<string>>()
  for (const edge of edges) {
    if (!adjacency.has(edge.from_step_id)) adjacency.set(edge.from_step_id, new Set())
    if (!adjacency.has(edge.to_step_id)) adjacency.set(edge.to_step_id, new Set())
    adjacency.get(edge.from_step_id)!.add(edge.to_step_id)
    adjacency.get(edge.to_step_id)!.add(edge.from_step_id)
  }

  const result = new Map<string, ProtocolGroupEntry>()

  for (const protocolId of protocolStepIds) {
    const step = steps.find((s) => s.id === protocolId)
    const protocolInfo = protocolsByStep.get(protocolId)
    const protocolType = protocolInfo?.protocol_type ?? step?.execution_mode ?? 'default'
    const color = PROTOCOL_TYPE_COLORS[protocolType] ?? GREYSCALE_ACCENT

    // BFS from protocol step
    const visited = new Set<string>([protocolId])
    const queue = [protocolId]
    while (queue.length > 0) {
      const current = queue.shift()!
      const neighbors = adjacency.get(current)
      if (!neighbors) continue
      for (const neighbor of neighbors) {
        if (visited.has(neighbor)) continue
        visited.add(neighbor)
        if (protocolStepIds.has(neighbor)) continue
        result.set(neighbor, { protocolColor: color, protocolStepId: protocolId })
        queue.push(neighbor)
      }
    }
  }

  return result
}

export { computeProtocolGroups, isDocumenterStep }
