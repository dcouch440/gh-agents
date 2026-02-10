import type {Node, Edge} from "@xyflow/react";
import type {WorkflowStep, WorkflowStepEdge} from "@/types/workflow";
import {Collections} from "@/utils/collections";
import {FORM_NODE} from "./CanvasFormNode";
import {DOCUMENT_NODE} from "./DocumentNode";
import type {DocumentNodeData} from "./DocumentNode";

type ProtocolStepInfo = {
  protocol_type: string;
  name: string;
  portNames: string[];
};

type StepNodeData = {
  label: string;
  stepType: string;
  agentId: string | null;
  promptTemplateId: string | null;
  outputSchemaId: string | null;
  agentName: string | null;
  modelId: string | null;
  outputSchemaName: string | null;
  upstreamStepNames: string[];
  toolNames: string[];
  protocolType: string | null;
  protocolName: string | null;
  protocolPortNames: string[];
};

type StepNodeLookups = {
  agents: ReadonlyMap<string, {name: string; model_id: string}>;
  outputSchemas: ReadonlyMap<string, {name: string}>;
  stepNames: ReadonlyMap<string, string>;
  edges: ReadonlyArray<{from_step_id: string; to_step_id: string}>;
  toolsByAgent: ReadonlyMap<string, string[]>;
  protocolsByStep: ReadonlyMap<string, ProtocolStepInfo>;
};

const toRFNodes = (
  steps: WorkflowStep[],
  lookups: StepNodeLookups,
): Node[] => {
  const edgesByTarget = Collections.groupBy(lookups.edges, (e) => e.to_step_id);

  return steps.map((step): Node => {
    // Entry / document nodes
    const isDocumentNode =
      step.execution_mode === "entry" || step.execution_mode === "document";
    if (isDocumentNode) {
      const mode = step.execution_mode === "entry" ? "entry" : "document";
      const docData: DocumentNodeData = {
        label:
          step.name ??
          (mode === "entry" ? "Port of Entry" : "Document"),
        mode,
        content: step.prompt_template,
      };
      return {
        id: step.id,
        type: "documentNode",
        position: {x: step.position_x ?? 0, y: step.position_y ?? 0},
        style: {
          width: DOCUMENT_NODE.DEFAULT_WIDTH,
          height: DOCUMENT_NODE.DEFAULT_HEIGHT,
        },
        data: docData,
      };
    }

    const agent = step.agent_id ? lookups.agents.get(step.agent_id) : undefined;
    const schema = step.output_schema_id
      ? lookups.outputSchemas.get(step.output_schema_id)
      : undefined;
    const upstreamEdges = edgesByTarget.get(step.id) ?? [];
    const upstreamStepNames = upstreamEdges.map(
      (e) => lookups.stepNames.get(e.from_step_id) ?? "Unknown Step",
    );
    const toolNames = step.agent_id
      ? (lookups.toolsByAgent.get(step.agent_id) ?? [])
      : [];
    const protocolInfo = lookups.protocolsByStep.get(step.id);

    const isDocumenter =
      protocolInfo?.protocol_type === "documenter" ||
      step.execution_mode === "documenter";

    if (isDocumenter) {
      return {
        id: step.id,
        type: "documenterNode",
        position: {x: step.position_x ?? 0, y: step.position_y ?? 0},
        style: {
          width: FORM_NODE.DEFAULT_WIDTH,
          height: FORM_NODE.DEFAULT_HEIGHT,
        },
        data: {
          label: step.name ?? "Documenter Protocol",
          documentNames: [],
          upstreamStepNames,
          promptValue: step.prompt_template,
          documents: [],
          modelId: agent?.model_id ?? null,
          agentName: agent?.name ?? null,
        },
      };
    }

    return {
      id: step.id,
      type: "stepNode",
      position: {x: step.position_x ?? 0, y: step.position_y ?? 0},
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
    };
  });
};

const toRFEdges = (edges: WorkflowStepEdge[]): Edge[] =>
  edges.map((edge) => ({
    id: edge.id,
    type: "stepEdge",
    source: edge.from_step_id,
    target: edge.to_step_id,
  }));

export {toRFNodes, toRFEdges};
export type {StepNodeData, StepNodeLookups, DocumentNodeData};
