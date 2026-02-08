import { useState, useCallback, useEffect, useMemo } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import InputBase from '@mui/material/InputBase'
import { PropertySection, PropertyRow, CodeEditor, PropertySelect } from '@/components/primitives'
import { useCollapsible, useDebounceCallback } from '@/hooks'
import { useStore, agentStore, promptTemplateStore, outputSchemaStore, workflowStore } from '@/stores'
import { DESIGN } from '@/constants'
import { STEP_TYPE_COLORS, DEFAULT_STEP_TYPE_COLOR } from '@/components/canvas/constants'
import type { WorkflowStep } from '@/types/workflow'

type StepPropertiesProps = {
  step: WorkflowStep
  readOnly?: boolean
}

function StepProperties({ step, readOnly = false }: StepPropertiesProps) {
  const general = useCollapsible(true)
  const agentSection = useCollapsible(true)
  const templateSection = useCollapsible(true)
  const schemaSection = useCollapsible(true)
  const promptSection = useCollapsible(true)

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
  const [prevStepId, setPrevStepId] = useState(step.id)

  if (prevStepId !== step.id) {
    setPrevStepId(step.id)
    setLocalName(step.name ?? '')
    setLocalPrompt(step.prompt_template)
  }

  // ── Save callbacks ──────────────────────────────────────────────────────────

  const debouncedSaveName = useDebounceCallback(
    (name: string) => { void workflowStore.updateStep(step.id, { name: name || undefined }) },
    500,
  )

  const debouncedSavePrompt = useDebounceCallback(
    (prompt: string) => { void workflowStore.updateStep(step.id, { prompt_template: prompt }) },
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

  // ── Mode badge color ────────────────────────────────────────────────────────

  const modeColor = STEP_TYPE_COLORS[step.execution_mode] ?? DEFAULT_STEP_TYPE_COLOR

  return (
    <Box>
      {/* Gradient accent bar */}
      <Box sx={{ height: 2, background: DESIGN.ACTIVE_GRADIENT, opacity: 0.6 }} />

      {/* ── General ────────────────────────────────────────────────────────── */}
      <PropertySection title="General" {...general}>
        <Box sx={{ px: '16px', pt: '4px', pb: '8px' }}>
          {readOnly ? (
            <Typography sx={{ fontSize: 13, fontWeight: 600, color: 'text.primary' }}>
              {step.name ?? 'Unnamed'}
            </Typography>
          ) : (
            <InputBase
              value={localName}
              onChange={(e) => { handleNameChange(e.target.value) }}
              placeholder="Unnamed"
              fullWidth
              sx={{
                fontSize: 13,
                fontWeight: 600,
                color: 'text.primary',
                px: '8px',
                py: '4px',
                borderRadius: '6px',
                border: 1,
                borderColor: 'transparent',
                transition: 'all 150ms ease',
                '&:hover': {
                  borderColor: 'divider',
                },
                '&.Mui-focused': {
                  borderColor: 'primary.main',
                  backgroundColor: DESIGN.ACTIVE_TINT,
                },
              }}
            />
          )}
        </Box>
        <PropertyRow label="Mode" last>
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
            <Box
              sx={{
                width: 6,
                height: 6,
                borderRadius: '50%',
                backgroundColor: modeColor,
              }}
            />
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
        </PropertyRow>
      </PropertySection>

      {/* ── Agent ──────────────────────────────────────────────────────────── */}
      <PropertySection title="Agent" {...agentSection}>
        <Box sx={{ px: '16px', pt: '4px', pb: '12px' }}>
          {readOnly ? (
            <PropertyRow
              label="Agent"
              value={agent?.name ?? step.agent_id}
              last
            />
          ) : (
            <PropertySelect
              value={step.agent_id}
              options={agentOptions}
              onChange={handleAgentChange}
              placeholder="Select agent..."
              accentColor={DESIGN.PORT_STRING}
            />
          )}
        </Box>
      </PropertySection>

      {/* ── Prompt Template ────────────────────────────────────────────────── */}
      <PropertySection title="Prompt Template" {...templateSection}>
        <Box sx={{ px: '16px', pt: '4px', pb: '12px' }}>
          {readOnly ? (
            <PropertyRow
              label="Template"
              value={templates.find((t) => t.id === step.prompt_template_id)?.name ?? 'None'}
              last
            />
          ) : (
            <PropertySelect
              value={step.prompt_template_id}
              options={templateOptions}
              onChange={handleTemplateChange}
              placeholder="Select template..."
              allowNone
              accentColor={DESIGN.PORT_JSON}
            />
          )}
        </Box>
      </PropertySection>

      {/* ── Output Schema ──────────────────────────────────────────────────── */}
      <PropertySection title="Output Schema" {...schemaSection}>
        <Box sx={{ px: '16px', pt: '4px', pb: '12px' }}>
          {readOnly ? (
            <PropertyRow
              label="Schema"
              value={schemas.find((s) => s.id === step.output_schema_id)?.name ?? 'None'}
              last
            />
          ) : (
            <PropertySelect
              value={step.output_schema_id}
              options={schemaOptions}
              onChange={handleSchemaChange}
              placeholder="Select schema..."
              allowNone
              accentColor={DESIGN.PORT_ARRAY}
            />
          )}
        </Box>
      </PropertySection>

      {/* ── System Prompt ──────────────────────────────────────────────────── */}
      <PropertySection title="System Prompt" {...promptSection}>
        <Box sx={{ px: '16px', pt: '4px', pb: '12px' }}>
          <CodeEditor
            key={step.id}
            value={localPrompt}
            onChange={handlePromptChange}
            language="markdown"
            placeholder="Enter system prompt..."
            height="200px"
            readOnly={readOnly}
          />
        </Box>
      </PropertySection>
    </Box>
  )
}

export { StepProperties }
export type { StepPropertiesProps }
