import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { PropertyRow, PropertySelect, AccentBarRow } from '@/components/primitives'
import { DESIGN } from '@/constants'
import { STEP_TYPE_COLORS, DEFAULT_STEP_TYPE_COLOR } from '@/components/canvas/constants'
import { SECTION_LABEL_SX, SCHEMA_PREVIEW_SX } from './constants'
import type { WorkflowStep } from '@/types/workflow'
import type { OutputSchema } from '@/types/schema'
import type { PropertySelectOption } from '@/components/primitives'

type OutputTabProps = {
  step: WorkflowStep
  readOnly: boolean
  downstreamSteps: WorkflowStep[]
  selectedSchema: OutputSchema | undefined
  schemaOptions: PropertySelectOption[]
  onSchemaChange: (schemaId: string | null) => void
}

function OutputTab({ step, readOnly, downstreamSteps, selectedSchema, schemaOptions, onSchemaChange }: OutputTabProps) {
  return (
    <Box sx={{ flex: 1, overflow: 'auto' }}>
      {/* Outgoing connections */}
      {downstreamSteps.length > 0 ? (
        <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
          <Typography sx={SECTION_LABEL_SX}>Outgoing</Typography>
          {downstreamSteps.map((s) => (
            <AccentBarRow
              key={s.id}
              barColor={STEP_TYPE_COLORS[s.execution_mode] ?? DEFAULT_STEP_TYPE_COLOR}
              primary={s.name ?? 'Unnamed'}
              secondary={s.execution_mode}
            />
          ))}
        </Box>
      ) : null}

      {/* Schema selector */}
      {readOnly ? (
        <Box sx={{ px: '16px', py: '10px' }}>
          <PropertyRow label="Schema" value={selectedSchema?.name ?? 'None'} />
        </Box>
      ) : (
        <Box sx={{ pb: '4px' }}>
          <Typography
            sx={{
              fontSize: 10,
              fontWeight: 500,
              color: 'text.secondary',
              textTransform: 'uppercase',
              letterSpacing: '0.04em',
              px: '16px',
              pt: '10px',
              pb: '2px',
            }}
          >
            Output Schema
          </Typography>
          <PropertySelect
            value={step.output_schema_id}
            options={schemaOptions}
            onChange={onSchemaChange}
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
            color: 'text.disabled',
            fontStyle: 'italic',
            px: '16px',
            pt: '8px',
          }}
        >
          No output schema selected
        </Typography>
      )}
    </Box>
  )
}

export { OutputTab }
export type { OutputTabProps }
