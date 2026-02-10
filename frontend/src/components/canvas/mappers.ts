import type {Node, Edge} from "@xyflow/react";
import type {WorkflowStep, WorkflowStepEdge} from "@/types/workflow";
import {FORM_NODE} from "./CanvasFormNode";

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
): Node<StepNodeData>[] => {
  const upstreamMap = new Map<string, string[]>();
  for (const edge of lookups.edges) {
    const list = upstreamMap.get(edge.to_step_id) ?? [];
    list.push(edge.from_step_id);
    upstreamMap.set(edge.to_step_id, list);
  }

  return steps.map((step) => {
    const agent = step.agent_id ? lookups.agents.get(step.agent_id) : undefined;
    const schema = step.output_schema_id
      ? lookups.outputSchemas.get(step.output_schema_id)
      : undefined;
    const upstreamIds = upstreamMap.get(step.id) ?? [];
    const upstreamStepNames = upstreamIds.map(
      (id) => lookups.stepNames.get(id) ?? "Unknown Step",
    );
    const toolNames = step.agent_id
      ? (lookups.toolsByAgent.get(step.agent_id) ?? [])
      : [];
    const protocolInfo = lookups.protocolsByStep.get(step.id);

    const isDocumenter =
      protocolInfo?.protocol_type === "documenter" ||
      step.execution_mode === "documenter";
    // todo: Make the data a switch case helper function.
    return {
      id: step.id,
      type: isDocumenter ? "documenterNode" : "stepNode",
      position: {x: step.position_x ?? 0, y: step.position_y ?? 0},
      ...(isDocumenter
        ? {
            style: {
              width: FORM_NODE.DEFAULT_WIDTH,
              height: FORM_NODE.DEFAULT_HEIGHT,
            },
          }
        : {}),
      data: isDocumenter
        ? {
            label: step.name ?? "Documenter Protocol",
            documentNames: [],
            upstreamStepNames,
            promptValue: step.prompt_template,
            documents: [], // Document defs loaded dynamically when node is selected
            modelId: agent?.model_id ?? null,
            agentName: agent?.name ?? null,
          }
        : {
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
export type {StepNodeData, StepNodeLookups};
