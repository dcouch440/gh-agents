import type { Node } from '@xyflow/react'
import type { WorkflowStep } from '@/types/workflow'
import { Collections } from '@/utils/collections'
import { FORM_NODE } from '../CanvasFormNode'
import { CONTEXT_NODE } from '../ContextNode'
import type { ContextNodeData } from '../ContextNode'
import { DOCUMENT_NODE } from '../DocumentNode'
import type { DocumentNodeData } from '../DocumentNode'
import { CanvasNodeKind } from '../canvasKinds'
import type { StepNodeLookups } from './types'
import { isDocumenterStep } from './protocolGroups'

const toRFNodes = (steps: WorkflowStep[], lookups: StepNodeLookups): Node[] => {
  const edgesByTarget = Collections.groupBy(lookups.edges, (e) => e.to_step_id)

  const stepNodes = steps.map((step): Node => {
    // Context nodes
    if (step.execution_mode === 'context') {
      const groupEntry = lookups.protocolGroups.get(step.id)
      const contextData: ContextNodeData = {
        kind: CanvasNodeKind.CONTEXT,
        label: step.name ?? 'Context',
        content: step.prompt_template,
        protocolColor: groupEntry?.protocolColor ?? null,
        protocolStepId: groupEntry?.protocolStepId ?? null,
      }
      return {
        id: step.id,
        type: 'contextNode',
        position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
        style: {
          width: CONTEXT_NODE.DEFAULT_WIDTH,
          height: CONTEXT_NODE.DEFAULT_HEIGHT,
        },
        data: contextData,
      }
    }

    const agent = step.agent_id ? lookups.agents.get(step.agent_id) : undefined
    const schema = step.output_schema_id ? lookups.outputSchemas.get(step.output_schema_id) : undefined
    const upstreamEdges = edgesByTarget.get(step.id) ?? []
    const upstreamStepNames = upstreamEdges.map((e) => lookups.stepNames.get(e.from_step_id) ?? 'Unknown Step')
    const toolNames = step.agent_id ? (lookups.toolsByAgent.get(step.agent_id) ?? []) : []
    const protocolInfo = lookups.protocolsByStep.get(step.id)

    if (isDocumenterStep(step, lookups.protocolsByStep)) {
      return {
        id: step.id,
        type: 'documenterNode',
        position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
        style: {
          width: FORM_NODE.DEFAULT_WIDTH,
          height: FORM_NODE.DEFAULT_HEIGHT,
        },
        data: {
          kind: CanvasNodeKind.PROTOCOL,
          label: step.name ?? 'Documenter Protocol',
          documentNames: [],
          upstreamStepNames,
          promptValue: step.prompt_template,
          modelId: agent?.model_id ?? null,
          agentName: agent?.name ?? null,
          isProtocol: true,
        },
      }
    }

    const groupEntry = lookups.protocolGroups.get(step.id)
    return {
      id: step.id,
      type: 'stepNode',
      position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
      data: {
        kind: CanvasNodeKind.STEP,
        label: step.name ?? step.execution_mode,
        stepType: step.execution_mode,
        agentId: step.agent_id,
        promptTemplateId: step.prompt_template_id,
        outputSchemaId: step.output_schema_id,
        agentName: agent?.name ?? null,
        modelId: agent?.model_id ?? null,
        outputSchemaName: schema?.name ?? null,
        upstreamStepNames,
        toolNames,
        protocolType: protocolInfo?.protocol_type ?? null,
        protocolName: protocolInfo?.name ?? null,
        protocolPortNames: protocolInfo?.portNames ?? [],
        protocolColor: groupEntry?.protocolColor ?? null,
        protocolStepId: groupEntry?.protocolStepId ?? null,
        isProtocol: false,
      },
    }
  })

  // Auto-generate document nodes for each documenter's document defs
  const documentNodes: Node[] = []
  for (const step of steps) {
    if (!isDocumenterStep(step, lookups.protocolsByStep)) continue

    const defs = lookups.documentDefsByStep[step.id] ?? []
    for (let i = 0; i < defs.length; i++) {
      const def = defs[i]!
      const docData: DocumentNodeData = {
        kind: CanvasNodeKind.DOCUMENT,
        label: def.name,
        documenterName: step.name ?? 'Documenter',
        content: '',
        protocolStepId: step.id,
      }
      documentNodes.push({
        id: `doc-artifact-${def.id}`,
        type: 'documentNode',
        position: {
          x: (step.position_x ?? 0) + i * (DOCUMENT_NODE.DEFAULT_WIDTH + 20),
          y: (step.position_y ?? 0) - DOCUMENT_NODE.DEFAULT_HEIGHT - 40,
        },
        style: {
          width: DOCUMENT_NODE.DEFAULT_WIDTH,
          height: DOCUMENT_NODE.DEFAULT_HEIGHT,
        },
        draggable: true,
        connectable: false,
        data: docData,
      })
    }
  }

  return [...stepNodes, ...documentNodes]
}

export { toRFNodes }
