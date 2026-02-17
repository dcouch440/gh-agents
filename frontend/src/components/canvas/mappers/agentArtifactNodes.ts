import type { Node } from '@xyflow/react'
import type { WorkflowStep } from '@/types/workflow'
import { CanvasNodeKind } from '../canvasKinds'
import { Archetype, AGENT_DEFAULTS } from '../DynamicNode/archetypes'
import type { DynamicNodeData } from '../DynamicNode/DynamicNode'
import { getStoredDimensions, getStoredPosition } from '../nodeResizeStorage'
import type { StepNodeLookups, AgentPositionMap } from './types'
import { isWorkforceStep } from './protocolGroups'

const toAgentArtifactNodes = (
  steps: WorkflowStep[],
  lookups: StepNodeLookups,
): { nodes: Node[]; agentPositionByRosterId: AgentPositionMap } => {
  const agentNodes: Node[] = []
  const agentPositionByRosterId = new Map<string, { x: number; y: number }>()

  for (const step of steps) {
    if (!isWorkforceStep(step, lookups.protocolsByStep)) continue

    const roster = lookups.rosterByStep[step.id] ?? []
    for (let i = 0; i < roster.length; i++) {
      const agent = roster[i]!
      if (!agent.child_step_id) continue

      const agentData: DynamicNodeData = {
        kind: CanvasNodeKind.AGENT,
        archetype: Archetype.AGENT,
        label: agent.name,
        description: '',
        documentNames: [],
        rosterNames: [],
        roomId: null,
        upstreamStepNames: [],
        promptValue: '',
        modelId: null,
        agentName: null,
        rosterAgentId: agent.id,
        roleDescription: agent.role_description,
        capabilities: [],
        parentStepName: step.name ?? 'Workforce',
        protocolStepId: step.id,
      }
      const agentNodeId = `agent-artifact-${agent.id}`
      const agentDims = getStoredDimensions(agentNodeId)
      const agentPos = getStoredPosition(agentNodeId)
      const defaultPos = {
        x: (step.position_x ?? 0),
        y: (step.position_y ?? 0) - (i + 1) * (AGENT_DEFAULTS.DEFAULT_HEIGHT + 20),
      }
      const position = agentPos ?? defaultPos
      agentPositionByRosterId.set(agent.id, position)
      agentNodes.push({
        id: agentNodeId,
        type: 'dynamicNode',
        position,
        style: {
          width: agentDims?.width ?? AGENT_DEFAULTS.DEFAULT_WIDTH,
          height: agentDims?.height ?? AGENT_DEFAULTS.DEFAULT_HEIGHT,
        },
        draggable: true,
        connectable: false,
        data: agentData,
      })
    }
  }

  return { nodes: agentNodes, agentPositionByRosterId }
}

export { toAgentArtifactNodes }
