import { useState, useEffect, useMemo } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import InputBase from '@mui/material/InputBase'
import { PropertyRow, PropertySelect, TabSelector } from '@/components/primitives'
import { useStore, agentStore, promptTemplateStore, outputSchemaStore, workflowStore, protocolStore } from '@/stores'
import { Collections } from '@/utils/collections'
import { useTheme } from '@mui/material/styles'
import { DESIGN } from '@/constants'
import { STEP_TYPE_COLORS, DEFAULT_STEP_TYPE_COLOR } from '@/components/canvas/constants'
import { SystemTab, TemplateTab, InputTab, OutputTab, ProtocolTab } from './StepPropertiesTabs'
import { useStepFieldHandlers } from './useStepFieldHandlers'
import { useStepVariableContext } from './useStepVariableContext'
import type { WorkflowStep } from '@/types/workflow'
import type { TabOption } from '@/components/primitives'

type StepPropertiesProps = {
  step: WorkflowStep
  steps: WorkflowStep[]
  readOnly?: boolean
}

type StepTab = 'system' | 'template' | 'input' | 'output' | 'protocol'

const TAB_OPTIONS: TabOption[] = [
  { value: 'system', label: 'System' },
  { value: 'template', label: 'Template' },
  { value: 'input', label: 'Input' },
  { value: 'output', label: 'Output' },
  { value: 'protocol', label: 'Protocol' },
]

function StepProperties({ step, steps, readOnly = false }: StepPropertiesProps) {
  const theme = useTheme()
  const [activeTab, setActiveTab] = useState<StepTab>('template')

  // ── Store data ──────────────────────────────────────────────────────────────

  const agents = useStore(agentStore.store, agentStore.selectAll)
  const templates = useStore(promptTemplateStore.store, promptTemplateStore.selectAll)
  const schemas = useStore(outputSchemaStore.store, outputSchemaStore.selectAll)

  const agent = useStore(agentStore.store, step.agent_id ? agentStore.selectById(step.agent_id) : () => undefined)
  const allProtocols = useStore(protocolStore.store, protocolStore.selectAll)

  useEffect(() => {
    void agentStore.fetchAll()
    void promptTemplateStore.fetchIfStale()
    void outputSchemaStore.fetchIfStale()
    void protocolStore.fetchAll()
  }, [])

  // ── Lookup maps ────────────────────────────────────────────────────────────

  const stepsById = useMemo(() => Collections.keyBy(steps, (s) => s.id), [steps])
  const templatesMap = useMemo(() => Collections.keyBy(templates, (t) => t.id), [templates])
  const schemasMap = useMemo(() => Collections.keyBy(schemas, (s) => s.id), [schemas])
  const agentsById = useMemo(() => Collections.keyBy(agents, (a) => a.id), [agents])

  // ── Dropdown options ────────────────────────────────────────────────────────

  const agentOptions = useMemo(() => agents.map((a) => ({ value: a.id, label: a.name, secondary: a.model_id })), [agents])

  const templateOptions = useMemo(
    () =>
      templates.map((t) => ({
        value: t.id,
        label: t.name,
        secondary: `${t.variables?.length ?? 0} variable(s)`,
      })),
    [templates],
  )

  const schemaOptions = useMemo(() => schemas.map((s) => ({ value: s.id, label: s.name })), [schemas])

  // ── Field handlers ─────────────────────────────────────────────────────────

  const { handleFieldChange, handleAgentChange, handleTemplateChange, handleSchemaChange, handleCopyVariable } =
    useStepFieldHandlers({ stepId: step.id, templatesMap })

  // ── Graph connections (derived from edges) ──────────────────────────────

  const edges = useStore(workflowStore.store, workflowStore.selectEdges)

  const incomingSteps = useMemo(
    () =>
      Collections.filterMap(edges, (e) => {
        if (e.to_step_id !== step.id) return null
        return stepsById.get(e.from_step_id) ?? null
      }),
    [edges, step.id, stepsById],
  )

  const downstreamSteps = useMemo(
    () =>
      Collections.filterMap(edges, (e) => {
        if (e.from_step_id !== step.id) return null
        return stepsById.get(e.to_step_id) ?? null
      }),
    [edges, step.id, stepsById],
  )

  const upstreamIds = useMemo(() => incomingSteps.map((s) => s.id), [incomingSteps])

  // ── Variable autocomplete ────────────────────────────────────────────────

  const { variableContext, autocompleteExtension } = useStepVariableContext({ upstreamIds, stepsById, schemasMap, step })

  // ── Mode badge color ────────────────────────────────────────────────────────

  const modeColor = STEP_TYPE_COLORS[step.execution_mode] ?? DEFAULT_STEP_TYPE_COLOR

  // ── Resolved output schema for the Output tab ─────────────────────────────

  const selectedSchema = step.output_schema_id ? schemasMap.get(step.output_schema_id) : undefined

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Gradient accent bar */}
      <Box
        sx={{
          height: 2,
          background: theme.palette.custom.activeGradient,
          opacity: 0.6,
          flexShrink: 0,
        }}
      />

      {/* ── Header: Name + Mode + Agent ──────────────────────────────── */}
      <Box sx={{ borderBottom: 1, borderColor: 'divider', flexShrink: 0 }}>
        <Box sx={{ px: '16px', pt: '12px', pb: '4px' }}>
          {readOnly ? (
            <Typography sx={{ fontSize: 14, fontWeight: 600, color: 'text.primary' }}>{step.name ?? 'Unnamed'}</Typography>
          ) : (
            <InputBase
              value={step.name ?? ''}
              onChange={(e) => {
                handleFieldChange('name', e.target.value)
              }}
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
                  backgroundColor: theme.palette.custom.activeTint,
                },
              }}
            />
          )}
        </Box>
        <Box
          sx={{
            px: '16px',
            pb: '8px',
            display: 'flex',
            alignItems: 'center',
            gap: 0.75,
          }}
        >
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
        </Box>

        {/* Agent selector — always visible in header */}
        {readOnly ? (
          <Box sx={{ px: '16px', pb: '10px' }}>
            <PropertyRow label="Agent" value={agent?.name ?? step.agent_id} />
          </Box>
        ) : (
          <Box sx={{ pb: '6px' }}>
            <Typography
              sx={{
                fontSize: 10,
                fontWeight: 500,
                color: 'text.secondary',
                textTransform: 'uppercase',
                letterSpacing: '0.04em',
                px: '16px',
                pb: '2px',
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
      <Box sx={{ flexShrink: 0, borderBottom: 1, borderColor: 'divider' }}>
        <TabSelector
          options={TAB_OPTIONS}
          value={activeTab}
          onChange={(v) => {
            setActiveTab(v as StepTab)
          }}
        />
      </Box>

      {/* ── Tab content ──────────────────────────────────────────────── */}
      <Box
        sx={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          minHeight: 0,
          overflow: 'hidden',
        }}
      >
        {activeTab === 'system' ? (
          <SystemTab
            step={step}
            agent={agent}
            readOnly={readOnly}
            completions={variableContext.completions}
            onCopyVariable={handleCopyVariable}
            onFieldChange={handleFieldChange}
            autocompleteExtension={autocompleteExtension}
          />
        ) : null}
        {activeTab === 'template' ? (
          <TemplateTab
            step={step}
            readOnly={readOnly}
            templatesMap={templatesMap}
            templateOptions={templateOptions}
            completions={variableContext.completions}
            onCopyVariable={handleCopyVariable}
            onFieldChange={handleFieldChange}
            onTemplateChange={handleTemplateChange}
            autocompleteExtension={autocompleteExtension}
          />
        ) : null}
        {activeTab === 'input' ? <InputTab incomingSteps={incomingSteps} schemasMap={schemasMap} /> : null}
        {activeTab === 'output' ? (
          <OutputTab
            step={step}
            readOnly={readOnly}
            downstreamSteps={downstreamSteps}
            selectedSchema={selectedSchema}
            schemaOptions={schemaOptions}
            onSchemaChange={handleSchemaChange}
          />
        ) : null}
        {activeTab === 'protocol' ? <ProtocolTab allProtocols={allProtocols} agentsById={agentsById} /> : null}
      </Box>
    </Box>
  )
}

export { StepProperties }
export type { StepPropertiesProps }
