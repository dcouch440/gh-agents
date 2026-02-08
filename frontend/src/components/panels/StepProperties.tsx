import Box from '@mui/material/Box'
import IconButton from '@mui/material/IconButton'
import OpenInNewOutlined from '@mui/icons-material/OpenInNewOutlined'
import { PropertySection, PropertyRow, AccentBarRow } from '@/components/primitives'
import { useCollapsible } from '@/hooks'
import { useStore, agentStore, promptTemplateStore, outputSchemaStore, layoutStore } from '@/stores'
import { DESIGN } from '@/constants'
import type { WorkflowStep } from '@/types/workflow'

type StepPropertiesProps = {
  step: WorkflowStep
}

function StepProperties({ step }: StepPropertiesProps) {
  const general = useCollapsible(true)
  const agentSection = useCollapsible(true)
  const promptSection = useCollapsible(true)
  const schemaSection = useCollapsible(true)
  const positionSection = useCollapsible(false)

  const agent = useStore(
    agentStore.store,
    step.agent_id ? agentStore.selectById(step.agent_id) : () => undefined,
  )

  const template = useStore(
    promptTemplateStore.store,
    step.prompt_template_id ? promptTemplateStore.selectById(step.prompt_template_id) : () => undefined,
  )

  const schema = useStore(
    outputSchemaStore.store,
    step.output_schema_id ? outputSchemaStore.selectById(step.output_schema_id) : () => undefined,
  )

  const navButton = (section: string) => (
    <IconButton
      size="small"
      onClick={() => { layoutStore.openRightPanel(section) }}
      sx={{ p: 0.25 }}
    >
      <OpenInNewOutlined sx={{ fontSize: 14, color: 'text.secondary' }} />
    </IconButton>
  )

  return (
    <Box>
      <PropertySection title="General" {...general}>
        <PropertyRow label="Name" value={step.name} />
        <PropertyRow label="Type" value={step.step_type} />
        <PropertyRow label="Description" value={step.description ?? 'None'} last />
      </PropertySection>

      <PropertySection title="Agent" {...agentSection}>
        {agent ? (
          <AccentBarRow
            barColor={DESIGN.PORT_STRING}
            primary={agent.name}
            secondary={agent.model_id}
            actions={navButton('agents')}
          />
        ) : (
          <PropertyRow label="Agent" value="None assigned" last />
        )}
      </PropertySection>

      <PropertySection title="Prompt Template" {...promptSection}>
        {template ? (
          <AccentBarRow
            barColor={DESIGN.PORT_JSON}
            primary={template.name}
            secondary={`${template.variables.length} variable(s)`}
            actions={navButton('prompts')}
          />
        ) : (
          <PropertyRow label="Template" value="None assigned" last />
        )}
      </PropertySection>

      <PropertySection title="Output Schema" {...schemaSection}>
        {schema ? (
          <AccentBarRow
            barColor={DESIGN.PORT_ARRAY}
            primary={schema.name}
            actions={navButton('schemas')}
          />
        ) : (
          <PropertyRow label="Schema" value="None assigned" last />
        )}
      </PropertySection>

      <PropertySection title="Position" {...positionSection}>
        <PropertyRow label="X" value={String(step.position_x)} mono />
        <PropertyRow label="Y" value={String(step.position_y)} mono last />
      </PropertySection>
    </Box>
  )
}

export { StepProperties }
export type { StepPropertiesProps }
