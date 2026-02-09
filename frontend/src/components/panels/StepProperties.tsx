import {useState, useCallback, useEffect, useMemo, useRef} from "react";
import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import InputBase from "@mui/material/InputBase";
import {
  PropertyRow,
  CodeEditor,
  PropertySelect,
  AccentBarRow,
  TabSelector,
  EmptyState,
  VariableChipStrip,
} from "@/components/primitives";
import {
  useStore,
  agentStore,
  promptTemplateStore,
  outputSchemaStore,
  workflowStore,
  protocolStore,
} from "@/stores";
import {DESIGN} from "@/constants";
import {
  STEP_TYPE_COLORS,
  DEFAULT_STEP_TYPE_COLOR,
  PROTOCOL_TYPE_COLORS,
} from "@/components/canvas/constants";
import {buildVariableCompletions} from "@/utils/variableContext";
import {createVariableAutocomplete} from "@/utils/variableAutocomplete";
import type {Extension} from "@codemirror/state";
import type {VariableCompletion} from "@/utils/variableContext";
import type {WorkflowStep} from "@/types/workflow";
import type {TabOption} from "@/components/primitives";

type StepPropertiesProps = {
  step: WorkflowStep;
  steps: WorkflowStep[];
  readOnly?: boolean;
};

type StepTab = "system" | "template" | "input" | "output" | "protocol";

const TAB_OPTIONS: TabOption[] = [
  {value: "system", label: "System"},
  {value: "template", label: "Template"},
  {value: "input", label: "Input"},
  {value: "output", label: "Output"},
  {value: "protocol", label: "Protocol"},
];

const EDITOR_CONTAINER_SX = {
  flex: 1,
  borderTop: 1,
  borderColor: "divider",
  minHeight: 0,
  "& > div": {border: "none", borderRadius: 0, height: "100%"},
  "& .cm-editor": {height: "100%"},
  "& .cm-scroller": {overflow: "auto"},
  "& .cm-gutters": {
    backgroundColor: "transparent",
    border: "none",
  },
  "& .cm-lineNumbers .cm-gutterElement": {
    paddingLeft: "2px",
    paddingRight: "2px",
    minWidth: "unset",
    fontSize: 10,
    opacity: 0.35,
  },
  "& .cm-content": {paddingLeft: "1px"},
} as const;

const MUTED_EDITOR_CONTAINER_SX = {
  ...EDITOR_CONTAINER_SX,
  opacity: 0.5,
  backgroundColor: "rgba(255,255,255,0.01)",
} as const;

const SECTION_LABEL_SX = {
  fontSize: 9,
  fontWeight: 600,
  letterSpacing: "0.06em",
  textTransform: "uppercase",
  color: "text.disabled",
  px: "16px",
  pt: "10px",
  pb: "4px",
} as const;

const SCHEMA_PREVIEW_SX = {
  mx: "16px",
  mb: "12px",
  p: "10px",
  borderRadius: "6px",
  backgroundColor: "rgba(255,255,255,0.02)",
  border: 1,
  borderColor: "divider",
  fontSize: 11,
  fontFamily: "monospace",
  color: "text.secondary",
  whiteSpace: "pre-wrap",
  wordBreak: "break-word",
  overflow: "auto",
  maxHeight: 300,
} as const;

function StepProperties({step, steps, readOnly = false}: StepPropertiesProps) {
  const [activeTab, setActiveTab] = useState<StepTab>("template");

  // ── Store data ──────────────────────────────────────────────────────────────

  const agents = useStore(agentStore.store, agentStore.selectAll);
  const templates = useStore(
    promptTemplateStore.store,
    promptTemplateStore.selectAll,
  );
  const schemas = useStore(
    outputSchemaStore.store,
    outputSchemaStore.selectAll,
  );

  const agent = useStore(
    agentStore.store,
    step.agent_id ? agentStore.selectById(step.agent_id) : () => undefined,
  );
  const allProtocols = useStore(protocolStore.store, protocolStore.selectAll);

  useEffect(() => {
    void agentStore.fetchAll();
    void promptTemplateStore.fetchIfStale();
    void outputSchemaStore.fetchIfStale();
    void protocolStore.fetchAll();
  }, []);

  // ── Lookup maps ────────────────────────────────────────────────────────────

  const stepsById = useMemo(
    () => new Map(steps.map((s) => [s.id, s])),
    [steps],
  );

  const templatesMap = useMemo(
    () => new Map(templates.map((t) => [t.id, t])),
    [templates],
  );

  const schemasMap = useMemo(
    () => new Map(schemas.map((s) => [s.id, s])),
    [schemas],
  );

  // ── Dropdown options ────────────────────────────────────────────────────────

  const agentOptions = useMemo(
    () =>
      agents.map((a) => ({value: a.id, label: a.name, secondary: a.model_id})),
    [agents],
  );

  const templateOptions = useMemo(
    () =>
      templates.map((t) => ({
        value: t.id,
        label: t.name,
        secondary: `${t.variables?.length ?? 0} variable(s)`,
      })),
    [templates],
  );

  const schemaOptions = useMemo(
    () => schemas.map((s) => ({value: s.id, label: s.name})),
    [schemas],
  );

  // ── Field handlers ─────────────────────────────────────────────────────────
  // All edits patch the store only. Nothing hits the API until the user clicks
  // Save on the canvas toolbar.

  const handleFieldChange = useCallback(
    (field: "name" | "prompt_template" | "system_prompt_suffix", value: string) => {
      const storeValue = field === "prompt_template" ? value : value || null;
      workflowStore.patchStepLocal(step.id, {[field]: storeValue} as Partial<WorkflowStep>);
    },
    [step.id],
  );

  const handleAgentChange = useCallback(
    (agentId: string | null) => {
      if (agentId !== null) {
        workflowStore.patchStepLocal(step.id, {agent_id: agentId});
      }
    },
    [step.id],
  );

  const handleTemplateChange = useCallback(
    (templateId: string | null) => {
      const tpl = templateId ? templatesMap.get(templateId) : undefined;
      workflowStore.patchStepLocal(step.id, {
        prompt_template_id: templateId,
        prompt_template: tpl?.template ?? '',
      });
    },
    [step.id, templatesMap],
  );

  const handleSchemaChange = useCallback(
    (schemaId: string | null) => {
      workflowStore.patchStepLocal(step.id, {output_schema_id: schemaId});
    },
    [step.id],
  );

  const handleCopyVariable = useCallback((label: string) => {
    void navigator.clipboard.writeText(label);
  }, []);

  // ── Graph connections (derived from edges) ──────────────────────────────

  const edges = useStore(workflowStore.store, workflowStore.selectEdges);

  const incomingSteps = useMemo(
    () =>
      edges
        .filter((e) => e.to_step_id === step.id)
        .map((e) => stepsById.get(e.from_step_id))
        .filter((s): s is WorkflowStep => s !== undefined),
    [edges, step.id, stepsById],
  );

  const downstreamSteps = useMemo(
    () =>
      edges
        .filter((e) => e.from_step_id === step.id)
        .map((e) => stepsById.get(e.to_step_id))
        .filter((s): s is WorkflowStep => s !== undefined),
    [edges, step.id, stepsById],
  );

  const upstreamIds = useMemo(
    () => incomingSteps.map((s) => s.id),
    [incomingSteps],
  );

  // ── Variable autocomplete ────────────────────────────────────────────────
  // CodeMirror extensions must be stable (created once), but need access to
  // latest completions. We use a ref-based getter: the extension captures a
  // function that reads completionsRef.current lazily when autocomplete
  // triggers — never during render.

  const completionsRef = useRef<VariableCompletion[]>([]);

  const variableContext = useMemo(
    () => buildVariableCompletions(upstreamIds, stepsById, schemasMap, step),
    [upstreamIds, stepsById, schemasMap, step],
  );

  useEffect(() => {
    completionsRef.current = variableContext.completions;
  }, [variableContext]);

  // Auto-set output_variable_name on upstream steps that don't have one,
  // so the backend can resolve variable references at execution time.
  useEffect(() => {
    for (const { stepId, derivedName } of variableContext.autoNamed) {
      workflowStore.patchStepLocal(stepId, { output_variable_name: derivedName });
    }
  }, [variableContext.autoNamed]);

  // The getter reads completionsRef.current lazily at autocomplete-trigger time,
  // not during render. The useMemo just creates the stable extension wrapper.
  const autocompleteExtension = useMemo<Extension>(
    // eslint-disable-next-line react-hooks/refs -- lazy getter, ref read at autocomplete time only
    () => createVariableAutocomplete(() => completionsRef.current),
    [],
  );

  // ── Mode badge color ────────────────────────────────────────────────────────

  const modeColor =
    STEP_TYPE_COLORS[step.execution_mode] ?? DEFAULT_STEP_TYPE_COLOR;

  // ── Resolved output schema for the Output tab ─────────────────────────────

  const selectedSchema = step.output_schema_id
    ? schemasMap.get(step.output_schema_id)
    : undefined;

  return (
    <Box sx={{display: "flex", flexDirection: "column", height: "100%"}}>
      {/* Gradient accent bar */}
      <Box
        sx={{
          height: 2,
          background: DESIGN.ACTIVE_GRADIENT,
          opacity: 0.6,
          flexShrink: 0,
        }}
      />

      {/* ── Header: Name + Mode + Agent ──────────────────────────────── */}
      <Box sx={{borderBottom: 1, borderColor: "divider", flexShrink: 0}}>
        <Box sx={{px: "16px", pt: "12px", pb: "4px"}}>
          {readOnly ? (
            <Typography
              sx={{fontSize: 14, fontWeight: 600, color: "text.primary"}}
            >
              {step.name ?? "Unnamed"}
            </Typography>
          ) : (
            <InputBase
              value={step.name ?? ""}
              onChange={(e) => {
                handleFieldChange("name", e.target.value);
              }}
              placeholder="Unnamed"
              fullWidth
              sx={{
                fontSize: 14,
                fontWeight: 600,
                color: "text.primary",
                px: "8px",
                py: "2px",
                borderRadius: "6px",
                border: 1,
                borderColor: "transparent",
                transition: "all 150ms ease",
                "&:hover": {borderColor: "divider"},
                "&.Mui-focused": {
                  borderColor: "primary.main",
                  backgroundColor: DESIGN.ACTIVE_TINT,
                },
              }}
            />
          )}
        </Box>
        <Box
          sx={{
            px: "16px",
            pb: "8px",
            display: "flex",
            alignItems: "center",
            gap: 0.75,
          }}
        >
          <Box
            sx={{
              display: "inline-flex",
              alignItems: "center",
              gap: 0.5,
              px: "6px",
              py: "1px",
              borderRadius: "4px",
              backgroundColor: `${modeColor}15`,
            }}
          >
            <Box
              sx={{
                width: 6,
                height: 6,
                borderRadius: "50%",
                backgroundColor: modeColor,
              }}
            />
            <Typography
              sx={{
                fontSize: 10,
                fontWeight: 600,
                color: modeColor,
                textTransform: "uppercase",
                letterSpacing: "0.05em",
              }}
            >
              {step.execution_mode}
            </Typography>
          </Box>
        </Box>

        {/* Agent selector — always visible in header */}
        {readOnly ? (
          <Box sx={{px: "16px", pb: "10px"}}>
            <PropertyRow label="Agent" value={agent?.name ?? step.agent_id} />
          </Box>
        ) : (
          <Box sx={{pb: "6px"}}>
            <Typography
              sx={{
                fontSize: 10,
                fontWeight: 500,
                color: "text.secondary",
                textTransform: "uppercase",
                letterSpacing: "0.04em",
                px: "16px",
                pb: "2px",
              }}
            >
              Agent
            </Typography>
            <PropertySelect
              value={step.agent_id}
              options={agentOptions}
              onChange={handleAgentChange}
              placeholder="Select agent..."
              accentColor={DESIGN.PORT_STRING}
            />
          </Box>
        )}
      </Box>

      {/* ── Tab bar ──────────────────────────────────────────────────── */}
      <Box sx={{flexShrink: 0, borderBottom: 1, borderColor: "divider"}}>
        <TabSelector
          options={TAB_OPTIONS}
          value={activeTab}
          onChange={(v) => {
            setActiveTab(v as StepTab);
          }}
        />
      </Box>

      {/* ── Tab content ──────────────────────────────────────────────── */}
      <Box
        sx={{
          flex: 1,
          display: "flex",
          flexDirection: "column",
          minHeight: 0,
          overflow: "hidden",
        }}
      >
        {/* ── System tab ─────────────────────────────────────────────── */}
        {activeTab === "system" ? (
          <Box
            sx={{
              flex: 1,
              display: "flex",
              flexDirection: "column",
              minHeight: 0,
              overflow: "auto",
            }}
          >
            {/* Agent base system prompt (read-only, muted) */}
            <Typography sx={SECTION_LABEL_SX}>Agent System Prompt</Typography>
            {agent ? (
              <Box
                sx={{
                  ...MUTED_EDITOR_CONTAINER_SX,
                  flex: "none",
                  minHeight: 120,
                  maxHeight: 240,
                }}
              >
                <CodeEditor
                  key={`sys-base-${step.id}-${agent.id}`}
                  value={agent.system_prompt}
                  onChange={() => {}}
                  language="markdown"
                  placeholder="No system prompt defined on agent"
                  height="100%"
                  readOnly
                />
              </Box>
            ) : (
              <Box sx={{px: "16px", py: "12px"}}>
                <Typography
                  sx={{
                    fontSize: 11,
                    color: "text.disabled",
                    fontStyle: "italic",
                  }}
                >
                  Select an agent to view its system prompt.
                </Typography>
              </Box>
            )}

            {/* Divider */}
            <Box sx={{borderTop: 1, borderColor: "divider"}} />

            {/* Available variables */}
            <VariableChipStrip completions={variableContext.completions} onCopy={handleCopyVariable} />

            {/* Step-level extension (editable) */}
            <Typography sx={SECTION_LABEL_SX}>Step Extension</Typography>
            <Box sx={{...EDITOR_CONTAINER_SX, flex: 1, minHeight: 120}}>
              <CodeEditor
                key={`sys-ext-${step.id}`}
                value={step.system_prompt_suffix ?? ""}
                onChange={(v: string) => {
                  handleFieldChange("system_prompt_suffix", v);
                }}
                language="markdown"
                placeholder="Enter system prompt extension..."
                height="100%"
                readOnly={readOnly}
              />
            </Box>
          </Box>
        ) : null}

        {/* ── Template tab ───────────────────────────────────────────── */}
        {activeTab === "template" ? (
          <Box
            sx={{
              flex: 1,
              display: "flex",
              flexDirection: "column",
              minHeight: 0,
            }}
          >
            {/* Template selector */}
            {readOnly ? (
              step.prompt_template_id ? (
                <Box sx={{px: "16px", py: "8px"}}>
                  <PropertyRow
                    label="Template"
                    value={
                      templatesMap.get(step.prompt_template_id)?.name ??
                      "Unknown"
                    }
                  />
                </Box>
              ) : null
            ) : (
              <Box sx={{pb: "4px"}}>
                <Typography
                  sx={{
                    fontSize: 10,
                    fontWeight: 500,
                    color: "text.secondary",
                    textTransform: "uppercase",
                    letterSpacing: "0.04em",
                    px: "16px",
                    pt: "8px",
                    pb: "2px",
                  }}
                >
                  Template
                </Typography>
                <PropertySelect
                  value={step.prompt_template_id}
                  options={templateOptions}
                  onChange={handleTemplateChange}
                  placeholder="Select template..."
                  allowNone
                  accentColor={DESIGN.PORT_JSON}
                />
              </Box>
            )}

            {/* Available variables */}
            <VariableChipStrip completions={variableContext.completions} onCopy={handleCopyVariable} />

            {/* Editor */}
            <Box sx={EDITOR_CONTAINER_SX}>
              <CodeEditor
                key={`tpl-${step.id}`}
                value={step.prompt_template}
                onChange={(v: string) => {
                  handleFieldChange("prompt_template", v);
                }}
                language="markdown"
                placeholder="Enter prompt template..."
                height="100%"
                readOnly={readOnly}
                showLineNumbers
                extensions={[autocompleteExtension]}
              />
            </Box>
          </Box>
        ) : null}

        {/* ── Input tab ──────────────────────────────────────────────── */}
        {activeTab === "input" ? (
          <Box sx={{flex: 1, overflow: "auto"}}>
            {incomingSteps.length === 0 ? (
              <EmptyState message="No incoming connections" />
            ) : (
              incomingSteps.map((s) => {
                const upSchema = s.output_schema_id
                  ? schemasMap.get(s.output_schema_id)
                  : undefined;
                return (
                  <Box
                    key={s.id}
                    sx={{borderBottom: 1, borderColor: "divider"}}
                  >
                    <AccentBarRow
                      barColor={
                        STEP_TYPE_COLORS[s.execution_mode] ??
                        DEFAULT_STEP_TYPE_COLOR
                      }
                      primary={s.name ?? "Unnamed"}
                      secondary={s.execution_mode}
                    />
                    {upSchema ? (
                      <>
                        <Typography
                          sx={{
                            fontSize: 10,
                            fontWeight: 500,
                            color: "text.secondary",
                            px: "16px",
                            pb: "4px",
                          }}
                        >
                          {upSchema.name}
                        </Typography>
                        <Box component="pre" sx={SCHEMA_PREVIEW_SX}>
                          {JSON.stringify(upSchema.schema, null, 2)}
                        </Box>
                      </>
                    ) : (
                      <Typography
                        sx={{
                          fontSize: 10,
                          color: "text.disabled",
                          fontStyle: "italic",
                          px: "16px",
                          pb: "12px",
                        }}
                      >
                        No output schema
                      </Typography>
                    )}
                  </Box>
                );
              })
            )}
          </Box>
        ) : null}

        {/* ── Output tab ─────────────────────────────────────────────── */}
        {activeTab === "output" ? (
          <Box sx={{flex: 1, overflow: "auto"}}>
            {/* Outgoing connections */}
            {downstreamSteps.length > 0 ? (
              <Box sx={{borderBottom: 1, borderColor: "divider"}}>
                <Typography sx={SECTION_LABEL_SX}>Outgoing</Typography>
                {downstreamSteps.map((s) => (
                  <AccentBarRow
                    key={s.id}
                    barColor={
                      STEP_TYPE_COLORS[s.execution_mode] ??
                      DEFAULT_STEP_TYPE_COLOR
                    }
                    primary={s.name ?? "Unnamed"}
                    secondary={s.execution_mode}
                  />
                ))}
              </Box>
            ) : null}

            {/* Schema selector */}
            {readOnly ? (
              <Box sx={{px: "16px", py: "10px"}}>
                <PropertyRow
                  label="Schema"
                  value={selectedSchema?.name ?? "None"}
                />
              </Box>
            ) : (
              <Box sx={{pb: "4px"}}>
                <Typography
                  sx={{
                    fontSize: 10,
                    fontWeight: 500,
                    color: "text.secondary",
                    textTransform: "uppercase",
                    letterSpacing: "0.04em",
                    px: "16px",
                    pt: "10px",
                    pb: "2px",
                  }}
                >
                  Output Schema
                </Typography>
                <PropertySelect
                  value={step.output_schema_id}
                  options={schemaOptions}
                  onChange={handleSchemaChange}
                  placeholder="Select schema..."
                  allowNone
                  accentColor={DESIGN.PORT_ARRAY}
                />
              </Box>
            )}

            {/* Schema preview */}
            {selectedSchema ? (
              <Box component="pre" sx={SCHEMA_PREVIEW_SX}>
                {JSON.stringify(selectedSchema.schema, null, 2)}
              </Box>
            ) : (
              <Typography
                sx={{
                  fontSize: 10,
                  color: "text.disabled",
                  fontStyle: "italic",
                  px: "16px",
                  pt: "8px",
                }}
              >
                No output schema selected
              </Typography>
            )}
          </Box>
        ) : null}

        {/* ── Protocol tab ────────────────────────────────────────────── */}
        {activeTab === "protocol" ? (
          <Box sx={{flex: 1, overflow: "auto"}}>
            <Typography sx={SECTION_LABEL_SX}>Available Protocols</Typography>
            {allProtocols.length === 0 ? (
              <EmptyState message="No protocols available" />
            ) : (
              allProtocols.map((proto) => {
                const protoColor = PROTOCOL_TYPE_COLORS[proto.protocol_type] ?? DEFAULT_STEP_TYPE_COLOR;
                return (
                  <Box
                    key={proto.id}
                    sx={{
                      borderBottom: 1,
                      borderColor: "divider",
                      px: "16px",
                      py: "10px",
                    }}
                  >
                    <Box sx={{display: "flex", alignItems: "center", gap: 1, mb: 0.5}}>
                      <Box
                        sx={{
                          px: "6px",
                          py: "2px",
                          borderRadius: "4px",
                          backgroundColor: `${protoColor}20`,
                        }}
                      >
                        <Typography
                          sx={{
                            fontSize: 9,
                            fontWeight: 700,
                            textTransform: "uppercase",
                            color: protoColor,
                            letterSpacing: "0.06em",
                            lineHeight: 1,
                          }}
                        >
                          {proto.protocol_type}
                        </Typography>
                      </Box>
                      <Typography
                        sx={{
                          fontSize: 12,
                          fontWeight: 600,
                          color: "text.primary",
                          flex: 1,
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                      >
                        {proto.name}
                      </Typography>
                    </Box>
                    <Typography
                      sx={{
                        fontSize: 11,
                        color: "text.secondary",
                        lineHeight: 1.4,
                        mb: proto.ports.length > 0 ? 1 : 0,
                      }}
                    >
                      {proto.description}
                    </Typography>
                    {proto.ports.length > 0 && (
                      <Box>
                        <Typography
                          sx={{
                            fontSize: 9,
                            fontWeight: 600,
                            textTransform: "uppercase",
                            color: "text.disabled",
                            letterSpacing: "0.06em",
                            mb: 0.5,
                          }}
                        >
                          Ports ({proto.ports.length})
                        </Typography>
                        <Box sx={{display: "flex", flexWrap: "wrap", gap: 0.5}}>
                          {proto.ports.map((port) => {
                            const portAgent = agents.find((a) => a.id === port.agent_id);
                            return (
                              <Box
                                key={port.id}
                                sx={{
                                  display: "inline-flex",
                                  alignItems: "center",
                                  gap: 0.5,
                                  px: "6px",
                                  py: "2px",
                                  borderRadius: "4px",
                                  backgroundColor: `${protoColor}10`,
                                  border: 1,
                                  borderColor: `${protoColor}25`,
                                }}
                              >
                                <Typography
                                  sx={{fontSize: 10, color: "text.secondary", fontWeight: 500}}
                                >
                                  {port.port_name}
                                </Typography>
                                {portAgent ? (
                                  <Typography
                                    sx={{fontSize: 9, color: "text.disabled"}}
                                  >
                                    {portAgent.name}
                                  </Typography>
                                ) : null}
                              </Box>
                            );
                          })}
                        </Box>
                      </Box>
                    )}
                    {proto.agent ? (
                      <Box sx={{mt: 0.75}}>
                        <Typography
                          sx={{fontSize: 10, color: "text.disabled"}}
                        >
                          Agent: {proto.agent.name} ({proto.agent.model_id})
                        </Typography>
                      </Box>
                    ) : null}
                  </Box>
                );
              })
            )}
          </Box>
        ) : null}
      </Box>
    </Box>
  );
}

export {StepProperties};
export type {StepPropertiesProps};
