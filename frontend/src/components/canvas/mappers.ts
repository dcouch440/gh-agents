import type { Node, Edge } from '@xyflow/react'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import { Collections } from '@/utils/collections'
import { FORM_NODE } from './CanvasFormNode'
import { CONTEXT_NODE } from './ContextNode'
import type { ContextNodeData } from './ContextNode'
import { DOCUMENT_NODE } from './DocumentNode'
import type { DocumentNodeData } from './DocumentNode'

type ProtocolStepInfo = {
  protocol_type: string
  name: string
  portNames: string[]
}

type StepNodeData = {
  label: string
  stepType: string
  agentId: string | null
  promptTemplateId: string | null
  outputSchemaId: string | null
  agentName: string | null
  modelId: string | null
  outputSchemaName: string | null
  upstreamStepNames: string[]
  toolNames: string[]
  protocolType: string | null
  protocolName: string | null
  protocolPortNames: string[]
}

type DocumentDefInfo = {
  id: string
  name: string
}

type StepNodeLookups = {
  agents: ReadonlyMap<string, { name: string; model_id: string }>
  outputSchemas: ReadonlyMap<string, { name: string }>
  stepNames: ReadonlyMap<string, string>
  edges: ReadonlyArray<{ from_step_id: string; to_step_id: string }>
  toolsByAgent: ReadonlyMap<string, string[]>
  protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>
  documentDefsByStep: Readonly<Record<string, ReadonlyArray<DocumentDefInfo>>>
}

const toRFNodes = (steps: WorkflowStep[], lookups: StepNodeLookups): Node[] => {
  const edgesByTarget = Collections.groupBy(lookups.edges, (e) => e.to_step_id)

  const stepNodes = steps.map((step): Node => {
    // Context nodes
    if (step.execution_mode === 'context') {
      const contextData: ContextNodeData = {
        label: step.name ?? 'Context',
        content: step.prompt_template,
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

    const isDocumenter = protocolInfo?.protocol_type === 'documenter' || step.execution_mode === 'documenter'

    if (isDocumenter) {
      return {
        id: step.id,
        type: 'documenterNode',
        position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
        style: {
          width: FORM_NODE.DEFAULT_WIDTH,
          height: FORM_NODE.DEFAULT_HEIGHT,
        },
        data: {
          label: step.name ?? 'Documenter Protocol',
          documentNames: [],
          upstreamStepNames,
          promptValue: step.prompt_template,
          modelId: agent?.model_id ?? null,
          agentName: agent?.name ?? null,
        },
      }
    }

    return {
      id: step.id,
      type: 'stepNode',
      position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
      data: {
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
      },
    }
  })

  // Auto-generate document nodes for each documenter's document defs
  const documentNodes: Node[] = []
  for (const step of steps) {
    const isDocumenter = step.execution_mode === 'documenter' || lookups.protocolsByStep.get(step.id)?.protocol_type === 'documenter'
    if (!isDocumenter) continue

    const defs = lookups.documentDefsByStep[step.id] ?? []
    for (let i = 0; i < defs.length; i++) {
      const def = defs[i]!
      const docData: DocumentNodeData = {
        label: def.name,
        documenterName: step.name ?? 'Documenter',
        content: '',
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

const toRFEdges = (edges: WorkflowStepEdge[]): Edge[] =>
  edges.map((edge) => ({
    id: edge.id,
    type: 'stepEdge',
    source: edge.from_step_id,
    target: edge.to_step_id,
  }))

const toDocumentEdges = (steps: WorkflowStep[], lookups: StepNodeLookups): Edge[] => {
  const edges: Edge[] = []
  for (const step of steps) {
    const isDocumenter = step.execution_mode === 'documenter' || lookups.protocolsByStep.get(step.id)?.protocol_type === 'documenter'
    if (!isDocumenter) continue

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

const nodeDataEqual = (a: Record<string, unknown>, b: Record<string, unknown>): boolean => {
  const keysA = Object.keys(a)
  if (keysA.length !== Object.keys(b).length) return false
  for (let i = 0; i < keysA.length; i++) {
    const key = keysA[i]!
    const valA = a[key]
    const valB = b[key]
    if (Array.isArray(valA)) {
      if (!Array.isArray(valB)) return false
      if (!Collections.arraysEqual(valA as readonly unknown[], valB as readonly unknown[])) return false
    } else {
      if (!Object.is(valA, valB)) return false
    }
  }
  return true
}

export { toRFNodes, toRFEdges, toDocumentEdges, nodeDataEqual }
export type { StepNodeData, StepNodeLookups, ContextNodeData }
