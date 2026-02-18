import type { Node } from '@xyflow/react'
import type { WorkflowStep } from '@/types/workflow'
import { Collections } from '@/utils/collections'
import { FORM_NODE } from '../CanvasFormNode'
import { CONTEXT_NODE } from '../ContextNode'
import type { ContextNodeData } from '../ContextNode'
import { INPUT_NODE } from '../InputNode'
import type { InputNodeData } from '../InputNode'
import { SUB_WORKFLOW_NODE } from '../SubWorkflowNode'
import type { SubWorkflowNodeData } from '../SubWorkflowNode'
import { CanvasNodeKind } from '../canvasKinds'
import { Archetype, ARCHETYPE_CONFIGS, resolveArchetype } from '../DynamicNode/archetypes'
import type { DynamicNodeData } from '../DynamicNode/DynamicNode'
import type { StepNodeLookups } from './types'
import { toAgentArtifactNodes } from './agentArtifactNodes'
import { toNotesArtifactNodes } from './notesArtifactNodes'

const toStepNodes = (steps: WorkflowStep[], lookups: StepNodeLookups): Node[] => {
  const edgesByTarget = Collections.groupBy(lookups.edges, (e) => e.to_step_id)

  return Collections.mapBy(steps, (step): Node => {
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

    // Sub-workflow nodes
    if (step.execution_mode === 'sub_workflow') {
      const subData: SubWorkflowNodeData = {
        kind: CanvasNodeKind.SUB_WORKFLOW,
        label: step.name ?? 'Sub-Workflow',
        templateName: null,
      }
      return {
        id: step.id,
        type: 'subWorkflowNode',
        position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
        style: {
          width: step.width ?? SUB_WORKFLOW_NODE.DEFAULT_WIDTH,
          height: step.height ?? SUB_WORKFLOW_NODE.DEFAULT_HEIGHT,
        },
        data: subData,
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
      const rosterAgents = lookups.rosterByStep[step.id] ?? []
      const dynamicData: DynamicNodeData = {
        kind: CanvasNodeKind.PROTOCOL,
        archetype,
        label: step.name ?? config.label,
        description: step.description,
        rosterNames: Collections.mapBy(rosterAgents, (a) => a.name),
        roomId: step.room_id ?? null,
        upstreamStepNames,
        promptValue: step.prompt_template,
        modelId: agent?.model_id ?? null,
        agentName: agent?.name ?? null,
        rosterAgentId: null,
        roleDescription: null,
        capabilities: [],
        parentStepName: null,
        protocolStepId: null,
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
}

const toRFNodes = (steps: WorkflowStep[], lookups: StepNodeLookups): Node[] => {
  const stepNodes = toStepNodes(steps, lookups)
  const { nodes: agentNodes } = toAgentArtifactNodes(steps, lookups)
  const notesNodes = toNotesArtifactNodes(steps, lookups)
  return [...stepNodes, ...agentNodes, ...notesNodes]
}

export { toRFNodes }
