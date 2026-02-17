import type { Node } from '@xyflow/react'
import type { WorkflowStep } from '@/types/workflow'
import { DOCUMENT_NODE } from '../DocumentNode'
import type { DocumentNodeData } from '../DocumentNode'
import { CanvasNodeKind } from '../canvasKinds'
import { AGENT_DEFAULTS } from '../DynamicNode/archetypes'
import { getStoredDimensions, getStoredPosition } from '../nodeResizeStorage'
import type { StepNodeLookups, AgentPositionMap } from './types'
import { isWorkforceStep } from './protocolGroups'

const toDocumentArtifactNodes = (
  steps: WorkflowStep[],
  lookups: StepNodeLookups,
  agentPositionByRosterId: AgentPositionMap,
): Node[] => {
  const documentNodes: Node[] = []

  for (const step of steps) {
    if (!isWorkforceStep(step, lookups.protocolsByStep)) continue

    const defs = lookups.documentDefsByStep[step.id] ?? []
    let unassignedIdx = 0
    for (const def of defs) {
      const docData: DocumentNodeData = {
        kind: CanvasNodeKind.DOCUMENT,
        label: def.name,
        parentStepName: step.name ?? 'Workforce',
        content: lookups.documentContentByDefId[def.id] ?? '',
        protocolStepId: step.id,
        documentId: def.document_id,
      }
      const docNodeId = `doc-artifact-${def.id}`
      const docDims = getStoredDimensions(docNodeId)
      const docPos = getStoredPosition(docNodeId)

      // Position to the right of the assigned agent, or fall back to above the protocol
      const assignedAgentPos = def.agent_roster_entry_id
        ? agentPositionByRosterId.get(def.agent_roster_entry_id)
        : undefined
      const defaultDocPos = assignedAgentPos
        ? { x: assignedAgentPos.x + AGENT_DEFAULTS.DEFAULT_WIDTH + 40, y: assignedAgentPos.y }
        : { x: (step.position_x ?? 0) + unassignedIdx * (DOCUMENT_NODE.DEFAULT_WIDTH + 20), y: (step.position_y ?? 0) - DOCUMENT_NODE.DEFAULT_HEIGHT - 40 }
      if (!assignedAgentPos) unassignedIdx++

      documentNodes.push({
        id: docNodeId,
        type: 'documentNode',
        position: docPos ?? defaultDocPos,
        style: {
          width: docDims?.width ?? DOCUMENT_NODE.DEFAULT_WIDTH,
          height: docDims?.height ?? DOCUMENT_NODE.DEFAULT_HEIGHT,
        },
        draggable: true,
        connectable: false,
        data: docData,
      })
    }
  }

  return documentNodes
}

export { toDocumentArtifactNodes }
