import type { Node } from '@xyflow/react'
import type { WorkflowStep } from '@/types/workflow'
import { Collections } from '@/utils/collections'
import { CanvasNodeKind } from '../canvasKinds'
import { ARCHETYPE_CONFIGS, VARIANT_CONFIGS, resolveArchetype, Archetype } from '../CanvasNode/registry'
import type { EditorNodeData, TabbedNodeData, CardNodeData, CompactNodeData } from '../CanvasNode/types'
import type { StepNodeLookups } from './types'
import { toAgentArtifactNodes } from './agentArtifactNodes'

const toStepNodes = (steps: WorkflowStep[], lookups: StepNodeLookups): Node[] => {
  const edgesByTarget = Collections.groupBy(lookups.edges, (e) => e.to_step_id)

  return Collections.mapBy(steps, (step): Node => {
    // Context nodes
    if (step.execution_mode === 'context') {
      const groupEntry = lookups.protocolGroups.get(step.id)
      const cfg = VARIANT_CONFIGS.context
      const data: EditorNodeData = {
        variant: 'context',
        kind: CanvasNodeKind.CONTEXT,
        label: step.name ?? 'Context',
        content: step.prompt_template,
        protocolColor: groupEntry?.protocolColor ?? null,
        protocolStepId: groupEntry?.protocolStepId ?? null,
      }
      return {
        id: step.id,
        type: 'canvasNode',
        position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
        style: {
          width: step.width ?? cfg.defaultWidth,
          height: step.height ?? cfg.defaultHeight,
        },
        data,
      }
    }

    // Input nodes
    if (step.execution_mode === 'input') {
      const groupEntry = lookups.protocolGroups.get(step.id)
      const cfg = VARIANT_CONFIGS.input
      const data: EditorNodeData = {
        variant: 'input',
        kind: CanvasNodeKind.INPUT,
        label: step.name ?? 'Input',
        content: step.prompt_template,
        protocolColor: groupEntry?.protocolColor ?? null,
        protocolStepId: groupEntry?.protocolStepId ?? null,
      }
      return {
        id: step.id,
        type: 'canvasNode',
        position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
        style: {
          width: step.width ?? cfg.defaultWidth,
          height: step.height ?? cfg.defaultHeight,
        },
        data,
      }
    }

    // Sub-workflow nodes
    if (step.execution_mode === 'sub_workflow') {
      const cfg = VARIANT_CONFIGS.sub_workflow
      const data: CompactNodeData = {
        variant: 'sub_workflow',
        kind: CanvasNodeKind.SUB_WORKFLOW,
        label: step.name ?? 'Sub-Workflow',
        templateName: null,
        protocolStepId: null,
      }
      return {
        id: step.id,
        type: 'canvasNode',
        position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
        style: {
          width: step.width ?? cfg.defaultWidth,
          height: step.height ?? cfg.defaultHeight,
        },
        data,
      }
    }

    const agent = step.agent_id ? lookups.agents.get(step.agent_id) : undefined
    const schema = step.output_schema_id ? lookups.outputSchemas.get(step.output_schema_id) : undefined
    const upstreamEdges = edgesByTarget.get(step.id) ?? []
    const upstreamStepNames = Collections.mapBy(upstreamEdges, (e) => lookups.stepNames.get(e.from_step_id) ?? 'Unknown Step')
    const toolNames = step.agent_id ? (lookups.toolsByAgent.get(step.agent_id) ?? []) : []
    const protocolInfo = lookups.protocolsByStep.get(step.id)

    // Route known archetypes to tabbed layout
    const archetype = resolveArchetype(step, lookups.protocolsByStep, step.id)
    if (archetype !== Archetype.BLANK) {
      const config = ARCHETYPE_CONFIGS[archetype]
      const rosterAgents = lookups.rosterByStep[step.id] ?? []
      const variant = archetype === Archetype.WORKFORCE ? 'workforce' as const
        : archetype === Archetype.MANAGER ? 'manager' as const
        : 'room' as const
      const cfg = VARIANT_CONFIGS[variant]
      const data: TabbedNodeData = {
        variant,
        kind: CanvasNodeKind.PROTOCOL,
        label: step.name ?? config.label,
        description: step.description,
        documentNames: [],
        rosterNames: Collections.mapBy(rosterAgents, (a) => a.name),
        roomId: step.room_id ?? null,
        upstreamStepNames,
        promptValue: step.prompt_template,
        modelId: agent?.model_id ?? null,
        agentName: agent?.name ?? null,
        protocolStepId: null,
      }
      return {
        id: step.id,
        type: 'canvasNode',
        position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
        style: {
          width: step.width ?? cfg.defaultWidth,
          height: step.height ?? cfg.defaultHeight,
        },
        data,
      }
    }

    // Fallback: card layout for single, for_each, etc.
    const groupEntry = lookups.protocolGroups.get(step.id)
    const data: CardNodeData = {
      variant: 'step',
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
    }
    return {
      id: step.id,
      type: 'canvasNode',
      position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
      data,
    }
  })
}

const toRFNodes = (steps: WorkflowStep[], lookups: StepNodeLookups): Node[] => {
  const stepNodes = toStepNodes(steps, lookups)
  const { nodes: agentNodes } = toAgentArtifactNodes(steps, lookups)
  return [...stepNodes, ...agentNodes]
}

export { toRFNodes }
