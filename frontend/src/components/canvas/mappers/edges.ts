import type { Edge } from '@xyflow/react'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import { PROTOCOL_TYPE_COLORS } from '../constants'
import type { ProtocolStepInfo, ProtocolGroupEntry, StepNodeLookups, StepEdgeData } from './types'
import { isDocumenterStep } from './protocolGroups'

const toRFEdges = (
  edges: WorkflowStepEdge[],
  protocolGroups: ReadonlyMap<string, ProtocolGroupEntry>,
  protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>,
  steps: readonly WorkflowStep[],
): Edge[] => {
  // Build lookup for documenter steps not in protocolsByStep (execution_mode fallback)
  const documenterStepIds = new Set<string>()
  for (let i = 0; i < steps.length; i++) {
    const step = steps[i]!
    if (!protocolsByStep.has(step.id) && step.execution_mode === 'documenter') {
      documenterStepIds.add(step.id)
    }
  }

  return edges.map((edge) => {
    // Edge is protocol-connected if either end is a protocol step or in a protocol group
    const sourceIsProtocol = protocolsByStep.has(edge.from_step_id)
    const targetIsProtocol = protocolsByStep.has(edge.to_step_id)
    const sourceGroup = protocolGroups.get(edge.from_step_id)
    const targetGroup = protocolGroups.get(edge.to_step_id)

    let protocolColor: string | null = null
    if (sourceIsProtocol) {
      const info = protocolsByStep.get(edge.from_step_id)!
      protocolColor = PROTOCOL_TYPE_COLORS[info.protocol_type] ?? null
    } else if (targetIsProtocol) {
      const info = protocolsByStep.get(edge.to_step_id)!
      protocolColor = PROTOCOL_TYPE_COLORS[info.protocol_type] ?? null
    } else if (documenterStepIds.has(edge.from_step_id) || documenterStepIds.has(edge.to_step_id)) {
      protocolColor = PROTOCOL_TYPE_COLORS['documenter'] ?? null
    } else if (sourceGroup) {
      protocolColor = sourceGroup.protocolColor
    } else if (targetGroup) {
      protocolColor = targetGroup.protocolColor
    }

    const data: StepEdgeData = { protocolColor }
    return {
      id: edge.id,
      type: 'stepEdge',
      source: edge.from_step_id,
      target: edge.to_step_id,
      data,
    }
  })
}

const toDocumentEdges = (steps: WorkflowStep[], lookups: StepNodeLookups): Edge[] => {
  const edges: Edge[] = []
  for (const step of steps) {
    if (!isDocumenterStep(step, lookups.protocolsByStep)) continue

    const defs = lookups.documentDefsByStep[step.id] ?? []
    for (const def of defs) {
      edges.push({
        id: `doc-edge-${def.id}`,
        type: 'documentEdge',
        source: step.id,
        sourceHandle: 'documents',
        target: `doc-artifact-${def.id}`,
        targetHandle: 'document-input',
        selectable: false,
        deletable: false,
      })
    }
  }
  return edges
}

export { toRFEdges, toDocumentEdges }
