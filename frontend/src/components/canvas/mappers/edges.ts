import type { Edge } from '@xyflow/react'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import { Collections } from '@/utils/collections'
import { STEP_TYPE_COLORS, GREYSCALE_ACCENT, PROTOCOL_TYPE_COLORS } from '../constants'
import { Archetype, ARCHETYPE_CONFIGS, resolveArchetype } from '../DynamicNode/archetypes'
import { NOTES_NODE } from '../NotesNode'
import type { ProtocolStepInfo, ProtocolGroupEntry, StepNodeLookups, StepEdgeData } from './types'
import { isWorkforceStep } from './protocolGroups'

/** Resolve the intrinsic color for a step based on its archetype or execution mode. */
const resolveStepColor = (
  step: WorkflowStep,
  protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>,
): string => {
  const archetype = resolveArchetype(step, protocolsByStep, step.id)
  if (archetype !== Archetype.BLANK) return ARCHETYPE_CONFIGS[archetype].color

  return STEP_TYPE_COLORS[step.execution_mode] ?? GREYSCALE_ACCENT
}

const toRFEdges = (
  edges: WorkflowStepEdge[],
  protocolGroups: ReadonlyMap<string, ProtocolGroupEntry>,
  protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>,
  steps: readonly WorkflowStep[],
): Edge[] => {
  const stepsById = Collections.keyBy(steps, (s) => s.id)

  return Collections.mapBy(edges, (edge) => {
    const sourceStep = stepsById.get(edge.from_step_id)
    const sourceColor = sourceStep
      ? resolveStepColor(sourceStep, protocolsByStep)
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

const toDocumentEdges = (steps: WorkflowStep[], lookups: StepNodeLookups): Edge[] => {
  const edges: Edge[] = []
  for (const step of steps) {
    if (!isWorkforceStep(step, lookups.protocolsByStep)) continue

    const roster = lookups.rosterByStep[step.id] ?? []
    const agentHasNode = Collections.toSet(
      Collections.filterMap(roster, (a) => a.child_step_id ? a.id : null),
    )

    const defs = lookups.documentDefsByStep[step.id] ?? []
    for (const def of defs) {
      // Wire from assigned agent when it has a canvas node, else from protocol
      const fromAgent = def.agent_roster_entry_id && agentHasNode.has(def.agent_roster_entry_id)
      edges.push({
        id: `doc-edge-${def.id}`,
        type: 'artifactEdge',
        source: fromAgent ? `agent-artifact-${def.agent_roster_entry_id}` : step.id,
        sourceHandle: fromAgent ? 'agent-documents' : 'documents',
        target: `doc-artifact-${def.id}`,
        targetHandle: 'document-input',
        data: { color: PROTOCOL_TYPE_COLORS['workforce'] },
        selectable: false,
        deletable: false,
      })
    }
  }
  return edges
}

const toNotesEdges = (steps: WorkflowStep[], lookups: StepNodeLookups): Edge[] => {
  const edges: Edge[] = []
  for (const step of steps) {
    if (step.execution_mode === 'context' || step.execution_mode === 'input') continue
    const content = lookups.notesByStep[step.id]
    if (!content) continue

    edges.push({
      id: `notes-edge-${step.id}`,
      type: 'artifactEdge',
      source: step.id,
      sourceHandle: 'notes',
      target: `notes-${step.id}`,
      targetHandle: 'notes-input',
      data: { color: NOTES_NODE.ACCENT_COLOR },
      selectable: false,
      deletable: false,
    })
  }
  return edges
}

const toAgentEdges = (steps: WorkflowStep[], lookups: StepNodeLookups): Edge[] => {
  const edges: Edge[] = []
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
          data: { color: ARCHETYPE_CONFIGS[Archetype.AGENT].color },
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
            data: { color: ARCHETYPE_CONFIGS[Archetype.AGENT].color },
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

export { toRFEdges, toDocumentEdges, toAgentEdges, toNotesEdges }
