import type { Node } from '@xyflow/react'
import type { WorkflowStep } from '@/types/workflow'
import { Collections } from '@/utils/collections'
import { FORM_NODE } from '../CanvasFormNode'
import { CONTEXT_NODE } from '../ContextNode'
import type { ContextNodeData } from '../ContextNode'
import { DOCUMENT_NODE } from '../DocumentNode'
import type { DocumentNodeData } from '../DocumentNode'
import { INPUT_NODE } from '../InputNode'
import type { InputNodeData } from '../InputNode'
import { NOTES_NODE } from '../NotesNode'
import type { NotesNodeData } from '../NotesNode'
import { CanvasNodeKind } from '../canvasKinds'
import { Archetype, ARCHETYPE_CONFIGS, resolveArchetype } from '../DynamicNode/archetypes'
import type { DynamicNodeData } from '../DynamicNode/DynamicNode'
import type { StepNodeLookups } from './types'
import { isDocumenterStep } from './protocolGroups'
import { getStoredDimensions, getStoredPosition } from '../nodeResizeStorage'

const toRFNodes = (steps: WorkflowStep[], lookups: StepNodeLookups): Node[] => {
  const edgesByTarget = Collections.groupBy(lookups.edges, (e) => e.to_step_id)

  const stepNodes = Collections.mapBy(steps, (step): Node => {
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
          width: step.width ?? CONTEXT_NODE.DEFAULT_WIDTH,
          height: step.height ?? CONTEXT_NODE.DEFAULT_HEIGHT,
        },
        data: contextData,
      }
    }

    // Input nodes
    if (step.execution_mode === 'input') {
      const groupEntry = lookups.protocolGroups.get(step.id)
      const inputData: InputNodeData = {
        kind: CanvasNodeKind.INPUT,
        label: step.name ?? 'Input',
        content: step.prompt_template,
        protocolColor: groupEntry?.protocolColor ?? null,
        protocolStepId: groupEntry?.protocolStepId ?? null,
      }
      return {
        id: step.id,
        type: 'inputNode',
        position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
        style: {
          width: step.width ?? INPUT_NODE.DEFAULT_WIDTH,
          height: step.height ?? INPUT_NODE.DEFAULT_HEIGHT,
        },
        data: inputData,
      }
    }

    const agent = step.agent_id ? lookups.agents.get(step.agent_id) : undefined
    const schema = step.output_schema_id ? lookups.outputSchemas.get(step.output_schema_id) : undefined
    const upstreamEdges = edgesByTarget.get(step.id) ?? []
    const upstreamStepNames = Collections.mapBy(upstreamEdges, (e) => lookups.stepNames.get(e.from_step_id) ?? 'Unknown Step')
    const toolNames = step.agent_id ? (lookups.toolsByAgent.get(step.agent_id) ?? []) : []
    const protocolInfo = lookups.protocolsByStep.get(step.id)

    // Route known archetypes to DynamicNode
    const archetype = resolveArchetype(step, lookups.protocolsByStep, step.id)
    if (archetype !== Archetype.BLANK) {
      const config = ARCHETYPE_CONFIGS[archetype]
      const docDefs = lookups.documentDefsByStep[step.id] ?? []
      const rosterAgents = lookups.rosterByStep[step.id] ?? []
      const dynamicData: DynamicNodeData = {
        kind: CanvasNodeKind.PROTOCOL,
        archetype,
        label: step.name ?? config.label,
        description: step.description,
        documentNames: Collections.mapBy(docDefs, (d) => d.name),
        rosterNames: Collections.mapBy(rosterAgents, (a) => a.name),
        roomId: step.room_id ?? null,
        upstreamStepNames,
        promptValue: step.prompt_template,
        modelId: agent?.model_id ?? null,
        agentName: agent?.name ?? null,
      }
      return {
        id: step.id,
        type: 'dynamicNode',
        position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
        style: {
          width: step.width ?? FORM_NODE.DEFAULT_WIDTH,
          height: step.height ?? FORM_NODE.DEFAULT_HEIGHT,
        },
        data: dynamicData,
      }
    }

    // Fallback: existing StepNode for single, for_each, etc.
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
        content: lookups.documentContentByDefId[def.id] ?? '',
        protocolStepId: step.id,
      }
      const docNodeId = `doc-artifact-${def.id}`
      const docDims = getStoredDimensions(docNodeId)
      const docPos = getStoredPosition(docNodeId)
      documentNodes.push({
        id: docNodeId,
        type: 'documentNode',
        position: docPos ?? {
          x: (step.position_x ?? 0) + i * (DOCUMENT_NODE.DEFAULT_WIDTH + 20),
          y: (step.position_y ?? 0) - DOCUMENT_NODE.DEFAULT_HEIGHT - 40,
        },
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

  // Auto-generate notes nodes for steps that have assistant notes
  const notesNodes: Node[] = []
  for (const step of steps) {
    if (step.execution_mode === 'context' || step.execution_mode === 'input') continue
    const content = lookups.notesByStep[step.id]
    if (!content) continue

    const notesData: NotesNodeData = {
      kind: CanvasNodeKind.NOTES,
      label: 'Agent Notes',
      stepName: step.name ?? step.execution_mode,
      content,
      protocolStepId: step.id,
    }
    const notesNodeId = `notes-${step.id}`
    const notesDims = getStoredDimensions(notesNodeId)
    const notesPos = getStoredPosition(notesNodeId)
    notesNodes.push({
      id: notesNodeId,
      type: 'notesNode',
      position: notesPos ?? {
        x: (step.position_x ?? 0),
        y: (step.position_y ?? 0) + NOTES_NODE.DEFAULT_HEIGHT + 40,
      },
      style: {
        width: notesDims?.width ?? NOTES_NODE.DEFAULT_WIDTH,
        height: notesDims?.height ?? NOTES_NODE.DEFAULT_HEIGHT,
      },
      draggable: true,
      connectable: false,
      data: notesData,
    })
  }

  return [...stepNodes, ...documentNodes, ...notesNodes]
}

export { toRFNodes }
