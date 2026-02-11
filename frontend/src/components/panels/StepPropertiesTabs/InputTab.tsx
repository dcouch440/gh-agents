import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { AccentBarRow, EmptyState } from '@/components/primitives'
import { STEP_TYPE_COLORS, DEFAULT_STEP_TYPE_COLOR } from '@/components/canvas/constants'
import { SCHEMA_PREVIEW_SX } from './constants'
import type { WorkflowStep } from '@/types/workflow'
import type { OutputSchema } from '@/types/schema'

type InputTabProps = {
  incomingSteps: WorkflowStep[]
  schemasMap: Map<string, OutputSchema>
}

function InputTab({ incomingSteps, schemasMap }: InputTabProps) {
  if (incomingSteps.length === 0) {
    return (
      <Box sx={{ flex: 1, overflow: 'auto' }}>
        <EmptyState message="No incoming connections" />
      </Box>
    )
  }

  return (
    <Box sx={{ flex: 1, overflow: 'auto' }}>
      {incomingSteps.map((s) => {
        const upSchema = s.output_schema_id ? schemasMap.get(s.output_schema_id) : undefined
        return (
          <Box key={s.id} sx={{ borderBottom: 1, borderColor: 'divider' }}>
            <AccentBarRow
              barColor={STEP_TYPE_COLORS[s.execution_mode] ?? DEFAULT_STEP_TYPE_COLOR}
              primary={s.name ?? 'Unnamed'}
              secondary={s.execution_mode}
            />
            {upSchema ? (
              <>
                <Typography
                  sx={{
                    fontSize: 10,
                    fontWeight: 500,
                    color: 'text.secondary',
                    px: '16px',
                    pb: '4px',
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
                  color: 'text.disabled',
                  fontStyle: 'italic',
                  px: '16px',
                  pb: '12px',
                }}
              >
                No output schema
              </Typography>
            )}
          </Box>
        )
      })}
    </Box>
  )
}

export { InputTab }
export type { InputTabProps }
