import type { Edge } from '@xyflow/react'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import { Collections } from '@/utils/collections'
import { GREYSCALE_ACCENT } from '../constants'
import { resolveVariant } from '../CanvasNode/registry'
import type { NodePalette } from '@/theme'
import type { ProtocolStepInfo, ProtocolGroupEntry, StepNodeLookups, StepEdgeData } from './types'
import { isWorkforceStep } from './protocolGroups'

/** Resolve the themed color for a step via its node variant. */
const resolveStepColor = (
  step: WorkflowStep,
  protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>,
  nodePalette: NodePalette,
): string => {
  const variant = resolveVariant(step, protocolsByStep, step.id)
  return nodePalette[variant]
}

const toRFEdges = (
  edges: WorkflowStepEdge[],
  protocolGroups: ReadonlyMap<string, ProtocolGroupEntry>,
  protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>,
  steps: readonly WorkflowStep[],
  nodePalette: NodePalette,
): Edge[] => {
  const stepsById = Collections.keyBy(steps, (s) => s.id)

  return Collections.mapBy(edges, (edge) => {
    const sourceStep = stepsById.get(edge.from_step_id)
    const sourceColor = sourceStep
      ? resolveStepColor(sourceStep, protocolsByStep, nodePalette)
      : GREYSCALE_ACCENT

    // Edge is protocol-connected if either end is a protocol step, in a protocol group, or a workforce step
    const isProtocolEdge =
      protocolsByStep.has(edge.from_step_id) ||
      protocolsByStep.has(edge.to_step_id) ||
      protocolGroups.has(edge.from_step_id) ||
      protocolGroups.has(edge.to_step_id) ||
      (sourceStep?.execution_mode === 'workforce') === true

    const data: StepEdgeData = { sourceColor, isProtocolEdge }
    return {
      id: edge.id,
      type: 'stepEdge',
      source: edge.from_step_id,
      target: edge.to_step_id,
      data,
    }
  })
}

const toAgentEdges = (steps: WorkflowStep[], lookups: StepNodeLookups, nodePalette: NodePalette): Edge[] => {
  const edges: Edge[] = []
  const agentColor = nodePalette.agent
  for (const step of steps) {
    if (!isWorkforceStep(step, lookups.protocolsByStep)) continue

    const roster = lookups.rosterByStep[step.id] ?? []
    const active = roster.filter((a) => a.child_step_id !== null)
    const activeIds = Collections.toSetBy(active, (a) => a.id)

    for (const agent of active) {
      // Filter depends_on to only active roster agents
      const deps = agent.depends_on.filter((id) => activeIds.has(id))

      if (deps.length === 0) {
        // Root agent → edge from protocol node
        edges.push({
          id: `agent-edge-${agent.id}`,
          type: 'artifactEdge',
          data: { color: agentColor },
          source: step.id,
          sourceHandle: 'agents',
          target: `agent-artifact-${agent.id}`,
          targetHandle: 'agent-input',
          selectable: false,
          deletable: false,
        })
      } else {
        // Non-root → edge from each dependency agent
        for (const depId of deps) {
          edges.push({
            id: `agent-dep-${depId}-${agent.id}`,
            type: 'artifactEdge',
            data: { color: agentColor },
            source: `agent-artifact-${depId}`,
            sourceHandle: 'agent-output',
            target: `agent-artifact-${agent.id}`,
            targetHandle: 'agent-input',
            selectable: false,
            deletable: false,
          })
        }
      }
    }
  }
  return edges
}

export { toRFEdges, toAgentEdges }
