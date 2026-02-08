import { useState, useCallback, useEffect, useMemo, useRef } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import InputBase from '@mui/material/InputBase'
import KeyboardArrowDownRounded from '@mui/icons-material/KeyboardArrowDownRounded'
import { PropertySection, PropertyRow, CodeEditor, PropertySelect, AccentBarRow } from '@/components/primitives'
import { useCollapsible, useDebounceCallback } from '@/hooks'
import { useStore, agentStore, promptTemplateStore, outputSchemaStore, workflowStore } from '@/stores'
import { DESIGN, ANIMATION } from '@/constants'
import { STEP_TYPE_COLORS, DEFAULT_STEP_TYPE_COLOR } from '@/components/canvas/constants'
import { buildVariableCompletions } from '@/utils/variableContext'
import { createVariableAutocomplete } from '@/utils/variableAutocomplete'
import { extractVariables } from '@/utils/variables'
import type { Extension } from '@codemirror/state'
import type { VariableCompletion } from '@/utils/variableContext'
import type { WorkflowStep } from '@/types/workflow'

type StepPropertiesProps = {
  step: WorkflowStep
  steps: WorkflowStep[]
  readOnly?: boolean
}

const EDITOR_CONTAINER_SX = {
  flex: 1,
  borderTop: 1,
  borderColor: 'divider',
  minHeight: 0,
  '& > div': { border: 'none', borderRadius: 0, height: '100%' },
  '& .cm-editor': { height: '100%' },
  '& .cm-scroller': { overflow: 'auto' },
  '& .cm-gutters': {
    backgroundColor: 'transparent',
    border: 'none',
  },
  '& .cm-lineNumbers .cm-gutterElement': {
    paddingLeft: '2px',
    paddingRight: '2px',
    minWidth: 'unset',
    fontSize: 10,
    opacity: 0.35,
  },
  '& .cm-content': { paddingLeft: '1px' },
} as const

function StepProperties({ step, steps, readOnly = false }: StepPropertiesProps) {
  const incomingSection = useCollapsible(true)
  const outgoingSection = useCollapsible(true)
  const configSection = useCollapsible(true)
  const systemPromptSection = useCollapsible(true)
  const templateSection = useCollapsible(true)

  // ── Store data ──────────────────────────────────────────────────────────────

  const agents = useStore(agentStore.store, agentStore.selectAll)
  const templates = useStore(promptTemplateStore.store, promptTemplateStore.selectAll)
  const schemas = useStore(outputSchemaStore.store, outputSchemaStore.selectAll)

  const agent = useStore(
    agentStore.store,
    step.agent_id ? agentStore.selectById(step.agent_id) : () => undefined,
  )

  useEffect(() => {
    void agentStore.fetchAll()
    void promptTemplateStore.fetchIfStale()
    void outputSchemaStore.fetchIfStale()
  }, [])

  // ── Local state for debounced fields ────────────────────────────────────────

  const [localName, setLocalName] = useState(step.name ?? '')
  const [localPrompt, setLocalPrompt] = useState(step.prompt_template)
  const [localSystemPrompt, setLocalSystemPrompt] = useState(step.system_prompt_suffix ?? '')
  const [localOutputVar, setLocalOutputVar] = useState(step.output_variable_name ?? '')

  // Reset local state when the selected step changes. Intentionally keyed on
  // step.id only — we do NOT want store updates from debounced saves to
  // overwrite in-progress edits.
  useEffect(() => {
    setLocalName(step.name ?? '')
    setLocalPrompt(step.prompt_template)
    setLocalSystemPrompt(step.system_prompt_suffix ?? '')
    setLocalOutputVar(step.output_variable_name ?? '')
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [step.id])

  // ── Save callbacks ──────────────────────────────────────────────────────────

  const debouncedSaveName = useDebounceCallback(
    (name: string) => { void workflowStore.updateStep(step.id, { name: name || undefined }) },
    500,
  )

  const debouncedSavePrompt = useDebounceCallback(
    (prompt: string) => { void workflowStore.updateStep(step.id, { prompt_template: prompt }) },
    500,
  )

  const debouncedSaveSystemPrompt = useDebounceCallback(
    (value: string) => { void workflowStore.updateStep(step.id, { system_prompt_suffix: value || undefined }) },
    500,
  )

  const debouncedSaveOutputVar = useDebounceCallback(
    (value: string) => { void workflowStore.updateStep(step.id, { output_variable_name: value || undefined }) },
    500,
  )

  const handleNameChange = useCallback((value: string) => {
    setLocalName(value)
    debouncedSaveName(value)
  }, [debouncedSaveName])

  const handlePromptChange = useCallback((value: string) => {
    setLocalPrompt(value)
    debouncedSavePrompt(value)
  }, [debouncedSavePrompt])

  const handleSystemPromptChange = useCallback((value: string) => {
    setLocalSystemPrompt(value)
    debouncedSaveSystemPrompt(value)
  }, [debouncedSaveSystemPrompt])

  const handleOutputVarChange = useCallback((value: string) => {
    // Sanitize to valid variable name chars: lowercase, underscores, digits
    const sanitized = value.replace(/[^a-z0-9_]/g, '')
    setLocalOutputVar(sanitized)
    debouncedSaveOutputVar(sanitized)
  }, [debouncedSaveOutputVar])

  const handleAgentChange = useCallback((agentId: string | null) => {
    if (agentId !== null) {
      void workflowStore.updateStep(step.id, { agent_id: agentId })
    }
  }, [step.id])

  const handleTemplateChange = useCallback((templateId: string | null) => {
    void workflowStore.updateStep(step.id, { prompt_template_id: templateId ?? undefined })
  }, [step.id])

  const handleSchemaChange = useCallback((schemaId: string | null) => {
    void workflowStore.updateStep(step.id, { output_schema_id: schemaId ?? undefined })
  }, [step.id])

  // ── Dropdown options ────────────────────────────────────────────────────────

  const agentOptions = useMemo(
    () => agents.map((a) => ({ value: a.id, label: a.name, secondary: a.model_id })),
    [agents],
  )

  const templateOptions = useMemo(
    () => templates.map((t) => ({ value: t.id, label: t.name, secondary: `${t.variables.length} variable(s)` })),
    [templates],
  )

  const schemaOptions = useMemo(
    () => schemas.map((s) => ({ value: s.id, label: s.name })),
    [schemas],
  )

  // ── Lookup maps ────────────────────────────────────────────────────────────

  const stepsById = useMemo(
    () => new Map(steps.map((s) => [s.id, s])),
    [steps],
  )

  const templatesMap = useMemo(
    () => new Map(templates.map((t) => [t.id, t])),
    [templates],
  )

  const schemasMap = useMemo(
    () => new Map(schemas.map((s) => [s.id, s])),
    [schemas],
  )

  // ── Graph connections (O(1) from store adjacency map) ────────────────────

  const upstreamIds = useStore(workflowStore.store, workflowStore.selectUpstream(step.id))
  const downstreamIds = useStore(workflowStore.store, workflowStore.selectDownstream(step.id))

  const incomingSteps = useMemo(
    () => upstreamIds.map((id) => stepsById.get(id)).filter((s): s is WorkflowStep => s !== undefined),
    [upstreamIds, stepsById],
  )

  const downstreamSteps = useMemo(
    () => downstreamIds.map((id) => stepsById.get(id)).filter((s): s is WorkflowStep => s !== undefined),
    [downstreamIds, stepsById],
  )

  // ── Variable autocomplete ────────────────────────────────────────────────
  // CodeMirror extensions must be stable (created once), but need access to
  // latest completions. We use a ref-based getter: the extension captures a
  // function that reads completionsRef.current lazily when autocomplete
  // triggers — never during render.

  const completionsRef = useRef<VariableCompletion[]>([])

  const variableCompletions = useMemo(
    () => buildVariableCompletions(upstreamIds, stepsById, schemasMap),
    [upstreamIds, stepsById, schemasMap],
  )

  useEffect(() => {
    completionsRef.current = variableCompletions
  }, [variableCompletions])

  const autocompleteExtension = useMemo<Extension>(
    () => createVariableAutocomplete(() => completionsRef.current),
    [], // stable: getter reads from ref, no deps needed
  )

  // ── Prompt validation ────────────────────────────────────────────────────

  const hasVariableRef = useMemo(
    () => extractVariables(localPrompt).length > 0,
    [localPrompt],
  )

  // ── Mode badge color ────────────────────────────────────────────────────────

  const modeColor = STEP_TYPE_COLORS[step.execution_mode] ?? DEFAULT_STEP_TYPE_COLOR

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', minHeight: '100%' }}>
      {/* Gradient accent bar */}
      <Box sx={{ height: 2, background: DESIGN.ACTIVE_GRADIENT, opacity: 0.6, flexShrink: 0 }} />

      {/* ── Header: Name + Mode ────────────────────────────────────────── */}
      <Box sx={{ borderBottom: 1, borderColor: 'divider', flexShrink: 0 }}>
        <Box sx={{ px: '16px', pt: '12px', pb: '4px' }}>
          {readOnly ? (
            <Typography sx={{ fontSize: 14, fontWeight: 600, color: 'text.primary' }}>
              {step.name ?? 'Unnamed'}
            </Typography>
          ) : (
            <InputBase
              value={localName}
              onChange={(e) => { handleNameChange(e.target.value) }}
              placeholder="Unnamed"
              fullWidth
              sx={{
                fontSize: 14,
                fontWeight: 600,
                color: 'text.primary',
                px: '8px',
                py: '2px',
                borderRadius: '6px',
                border: 1,
                borderColor: 'transparent',
                transition: 'all 150ms ease',
                '&:hover': { borderColor: 'divider' },
                '&.Mui-focused': {
                  borderColor: 'primary.main',
                  backgroundColor: DESIGN.ACTIVE_TINT,
                },
              }}
            />
          )}
        </Box>
        <Box sx={{ px: '16px', pb: '10px', display: 'flex', alignItems: 'center', gap: 0.75 }}>
          {/* Mode badge */}
          <Box
            sx={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 0.5,
              px: '6px',
              py: '1px',
              borderRadius: '4px',
              backgroundColor: `${modeColor}15`,
            }}
          >
            <Box sx={{ width: 6, height: 6, borderRadius: '50%', backgroundColor: modeColor }} />
            <Typography
              sx={{
                fontSize: 10,
                fontWeight: 600,
                color: modeColor,
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
              }}
            >
              {step.execution_mode}
            </Typography>
          </Box>
        </Box>
      </Box>

      {/* ── Incoming — upstream steps feeding into this one ──────────────── */}
      {incomingSteps.length > 0 ? (
        <PropertySection title="Incoming" {...incomingSection}>
          {incomingSteps.map((s) => (
            <AccentBarRow
              key={s.id}
              barColor={STEP_TYPE_COLORS[s.execution_mode] ?? DEFAULT_STEP_TYPE_COLOR}
              primary={s.name ?? 'Unnamed'}
              secondary={s.execution_mode}
            />
          ))}
        </PropertySection>
      ) : null}

      {/* ── Outgoing — downstream steps this one feeds ─────────────────── */}
      {downstreamSteps.length > 0 ? (
        <PropertySection title="Outgoing" {...outgoingSection}>
          {downstreamSteps.map((s) => (
            <AccentBarRow
              key={s.id}
              barColor={STEP_TYPE_COLORS[s.execution_mode] ?? DEFAULT_STEP_TYPE_COLOR}
              primary={s.name ?? 'Unnamed'}
              secondary={s.execution_mode}
            />
          ))}
        </PropertySection>
      ) : null}

      {/* ── Configuration (Agent + Schema) ──────────────────────────────── */}
      <PropertySection title="Configuration" {...configSection}>
        {readOnly ? (
          <Box sx={{ px: '16px', pt: '2px', pb: '12px', display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <PropertyRow label="Agent" value={agent?.name ?? step.agent_id} />
            <PropertyRow label="Schema" value={schemasMap.get(step.output_schema_id ?? '')?.name ?? 'None'} />
            <PropertyRow label="Output Variable" value={step.output_variable_name ?? 'None'} last />
          </Box>
        ) : (
          <Box sx={{ pt: '2px', pb: '4px', display: 'flex', flexDirection: 'column', gap: 0 }}>
            <Box>
              <Typography sx={{ fontSize: 10, fontWeight: 500, color: 'text.secondary', textTransform: 'uppercase', letterSpacing: '0.04em', px: '16px', pt: '6px', pb: '2px' }}>
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
            <Box>
              <Typography sx={{ fontSize: 10, fontWeight: 500, color: 'text.secondary', textTransform: 'uppercase', letterSpacing: '0.04em', px: '16px', pt: '6px', pb: '2px' }}>
                Schema
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
            <Box>
              <Typography sx={{ fontSize: 10, fontWeight: 500, color: 'text.secondary', textTransform: 'uppercase', letterSpacing: '0.04em', px: '16px', pt: '6px', pb: '2px' }}>
                Output Variable
              </Typography>
              <Box sx={{ px: '16px', pb: '4px' }}>
                <InputBase
                  value={localOutputVar}
                  onChange={(e) => { handleOutputVarChange(e.target.value) }}
                  placeholder="e.g. parse_output"
                  fullWidth
                  sx={{
                    fontSize: 12,
                    fontFamily: 'monospace',
                    color: 'text.primary',
                    px: '8px',
                    py: '3px',
                    borderRadius: '6px',
                    border: 1,
                    borderColor: 'divider',
                    transition: 'all 150ms ease',
                    '&:hover': { borderColor: 'text.disabled' },
                    '&.Mui-focused': {
                      borderColor: 'primary.main',
                      backgroundColor: DESIGN.ACTIVE_TINT,
                    },
                  }}
                />
              </Box>
            </Box>
          </Box>
        )}
      </PropertySection>

      {/* ── System Prompt ──────────────────────────────────────────────── */}
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          minHeight: 120,
          borderBottom: 1,
          borderColor: 'divider',
        }}
      >
        <Box
          onClick={systemPromptSection.onToggle}
          sx={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            px: '16px',
            pt: '12px',
            pb: '8px',
            cursor: 'pointer',
            userSelect: 'none',
          }}
        >
          <Box sx={{ display: 'flex', alignItems: 'baseline', gap: 1 }}>
            <Typography
              sx={{
                fontSize: 11,
                fontWeight: 700,
                letterSpacing: '0.06em',
                textTransform: 'uppercase',
                color: 'text.secondary',
                lineHeight: 1,
              }}
            >
              System Prompt
            </Typography>
            <Typography
              sx={{
                fontSize: 9,
                color: 'text.disabled',
                letterSpacing: '0.02em',
              }}
            >
              appends to agent prompt
            </Typography>
          </Box>
          <KeyboardArrowDownRounded
            sx={{
              fontSize: 16,
              color: 'text.disabled',
              transition: `transform ${ANIMATION.FAST}ms ease`,
              transform: systemPromptSection.open ? 'rotate(0deg)' : 'rotate(-90deg)',
            }}
          />
        </Box>
        {systemPromptSection.open ? (
          <Box sx={EDITOR_CONTAINER_SX}>
            <CodeEditor
              key={`sys-${step.id}`}
              value={localSystemPrompt}
              onChange={handleSystemPromptChange}
              language="markdown"
              placeholder="Enter system prompt suffix..."
              height="100%"
              readOnly={readOnly}
            />
          </Box>
        ) : null}
      </Box>

      {/* ── Prompt Template (fills remaining space) ────────────────────── */}
      <Box
        sx={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          minHeight: 200,
          borderBottom: 1,
          borderColor: 'divider',
        }}
      >
        <Box
          onClick={templateSection.onToggle}
          sx={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            px: '16px',
            pt: '12px',
            pb: '8px',
            cursor: 'pointer',
            userSelect: 'none',
          }}
        >
          <Typography
            sx={{
              fontSize: 11,
              fontWeight: 700,
              letterSpacing: '0.06em',
              textTransform: 'uppercase',
              color: 'text.secondary',
              lineHeight: 1,
            }}
          >
            Prompt Template
          </Typography>
          <KeyboardArrowDownRounded
            sx={{
              fontSize: 16,
              color: 'text.disabled',
              transition: `transform ${ANIMATION.FAST}ms ease`,
              transform: templateSection.open ? 'rotate(0deg)' : 'rotate(-90deg)',
            }}
          />
        </Box>
        {templateSection.open ? (
          <>
            {/* Template selector */}
            {readOnly ? (
              step.prompt_template_id ? (
                <Box sx={{ px: '16px', pb: '8px' }}>
                  <PropertyRow label="Template" value={templatesMap.get(step.prompt_template_id)?.name ?? 'Unknown'} />
                </Box>
              ) : null
            ) : (
              <Box sx={{ pb: '4px' }}>
                <Typography sx={{ fontSize: 10, fontWeight: 500, color: 'text.secondary', textTransform: 'uppercase', letterSpacing: '0.04em', px: '16px', pb: '2px' }}>
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

            {/* Validation warning */}
            {!readOnly && localPrompt.trim().length > 0 && !hasVariableRef ? (
              <Box sx={{ px: '16px', pb: '6px' }}>
                <Typography sx={{ fontSize: 10, color: 'warning.main', fontWeight: 500 }}>
                  {'No variable references found. Use { to insert upstream data.'}
                </Typography>
              </Box>
            ) : null}

            {/* Editor */}
            <Box sx={EDITOR_CONTAINER_SX}>
              <CodeEditor
                key={`tpl-${step.id}`}
                value={localPrompt}
                onChange={handlePromptChange}
                language="markdown"
                placeholder="Enter prompt template..."
                height="100%"
                readOnly={readOnly}
                showLineNumbers
                extensions={[autocompleteExtension]}
              />
            </Box>
          </>
        ) : null}
      </Box>
    </Box>
  )
}

export { StepProperties }
export type { StepPropertiesProps }
