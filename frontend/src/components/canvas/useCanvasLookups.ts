import { useMemo } from 'react'
import { Collections } from '@/utils/collections'
import { computeProtocolGroups } from './mappers'
import type { StepNodeLookups, ProtocolStepInfo, ProtocolGroupEntry } from './mappers'
import type { WorkflowStep, WorkflowStepEdge, DocumentDef, RosterAgent, Agent, OutputSchema, Tool } from '@/types'
import type { StepProtocolLink } from '@/stores'

type CanvasLookupsResult = {
  lookups: StepNodeLookups
  protocolGroups: ReadonlyMap<string, ProtocolGroupEntry>
  protocolsByStepLookup: ReadonlyMap<string, ProtocolStepInfo>
}

const useCanvasLookups = (
  steps: WorkflowStep[],
  edges: WorkflowStepEdge[],
  agents: Agent[],
  schemas: OutputSchema[],
  toolsByAgent: Record<string, Tool[]>,
  stepProtocols: Readonly<Record<string, StepProtocolLink>>,
  documentDefsByStep: Record<string, DocumentDef[]>,
  rosterByStep: Record<string, RosterAgent[]>,
  notesByStep: Record<string, string>,
): CanvasLookupsResult => {
  const agentLookup = useMemo(
    () =>
      Collections.toLookupMap(
        agents,
        (a) => a.id,
        (a) => ({ name: a.name, model_id: a.model_id }),
      ),
    [agents],
  )

  const schemaLookup = useMemo(
    () =>
      Collections.toLookupMap(
        schemas,
        (s) => s.id,
        (s) => ({ name: s.name }),
      ),
    [schemas],
  )

  const stepNameLookup = useMemo(
    () =>
      Collections.toLookupMap(
        steps,
        (s) => s.id,
        (s) => s.name ?? s.execution_mode,
      ),
    [steps],
  )

  const toolsByAgentLookup = useMemo(
    () =>
      Collections.toLookupMap(
        agents,
        (a) => a.id,
        (a) => {
          const tools = toolsByAgent[a.id] ?? []
          return Collections.mapBy(tools, (t) => t.name)
        },
      ),
    [agents, toolsByAgent],
  )

  const protocolsByStepLookup = useMemo(
    () =>
      Collections.toLookupMap(
        Object.entries(stepProtocols),
        ([stepId]) => stepId,
        ([, link]) => ({
          protocol_type: link.protocolType,
          name: link.protocolName,
          portNames: link.portNames,
        }),
      ),
    [stepProtocols],
  )

  const protocolGroups = useMemo(
    () => computeProtocolGroups(steps, edges, protocolsByStepLookup),
    [steps, edges, protocolsByStepLookup],
  )

  const lookups = useMemo(
    (): StepNodeLookups => ({
      agents: agentLookup,
      outputSchemas: schemaLookup,
      stepNames: stepNameLookup,
      edges,
      toolsByAgent: toolsByAgentLookup,
      protocolsByStep: protocolsByStepLookup,
      documentDefsByStep,
      rosterByStep,
      notesByStep,
      protocolGroups,
    }),
    [agentLookup, schemaLookup, stepNameLookup, edges, toolsByAgentLookup, protocolsByStepLookup, documentDefsByStep, rosterByStep, notesByStep, protocolGroups],
  )

  return { lookups, protocolGroups, protocolsByStepLookup }
}

export { useCanvasLookups }
export type { CanvasLookupsResult }
