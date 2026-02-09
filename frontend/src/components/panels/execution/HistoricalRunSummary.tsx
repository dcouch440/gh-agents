import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { StatusBadge, MarkdownPreview } from '@/components/primitives'
import type { WorkflowExecutionSummary } from '@/types'
import type { BadgeVariant } from '@/components/primitives/StatusBadge'

type HistoricalRunSummaryProps = {
  run: WorkflowExecutionSummary
}

const statusToBadge = (status: string): { label: string; variant: BadgeVariant } => {
  if (status === 'completed') return { label: 'Completed', variant: 'success' }
  if (status === 'failed') return { label: 'Failed', variant: 'error' }
  if (status === 'running') return { label: 'Running', variant: 'info' }
  return { label: status, variant: 'neutral' }
}

const formatDuration = (startedAt: string | null, completedAt: string | null): string | null => {
  if (!startedAt || !completedAt) return null
  const ms = new Date(completedAt).getTime() - new Date(startedAt).getTime()
  if (ms < 1000) return `${ms}ms`
  const seconds = ms / 1000
  if (seconds < 60) return `${seconds.toFixed(1)}s`
  const minutes = Math.floor(seconds / 60)
  const remainingSeconds = Math.round(seconds % 60)
  return `${minutes}m ${remainingSeconds}s`
}

const extractOutputText = (outputs: Record<string, unknown> | null): string | null => {
  if (!outputs) return null
  const entries = Object.entries(outputs)
  if (entries.length === 0) return null

  const parts: string[] = []
  for (const [key, value] of entries) {
    const label = key === '' ? 'Output' : key
    if (typeof value === 'object' && value !== null && 'response' in value) {
      parts.push(`### ${label}\n\n${(value as { response: string }).response}`)
    } else if (typeof value === 'string') {
      parts.push(`### ${label}\n\n${value}`)
    } else {
      parts.push(`### ${label}\n\n\`\`\`json\n${JSON.stringify(value, null, 2)}\n\`\`\``)
    }
  }
  return parts.join('\n\n---\n\n')
}

function HistoricalRunSummary({ run }: HistoricalRunSummaryProps) {
  const badge = statusToBadge(run.status)
  const duration = formatDuration(run.started_at, run.completed_at)
  const outputText = extractOutputText(run.outputs)

  return (
    <Box sx={{ flex: 1, overflow: 'auto', px: 2, py: 1.5 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1.5 }}>
        <StatusBadge label={badge.label} variant={badge.variant} />
        {duration && (
          <Typography variant="caption" sx={{ color: 'text.secondary' }}>
            {duration}
          </Typography>
        )}
        {run.started_at && (
          <Typography variant="caption" sx={{ color: 'text.disabled', ml: 'auto' }}>
            {new Date(run.started_at).toLocaleString()}
          </Typography>
        )}
      </Box>

      {run.error && (
        <Box
          sx={{
            p: 1.5,
            mb: 1.5,
            borderRadius: 1,
            bgcolor: 'error.main',
            color: 'error.contrastText',
            opacity: 0.9,
            fontFamily: 'monospace',
            fontSize: '0.75rem',
            whiteSpace: 'pre-wrap',
          }}
        >
          {run.error}
        </Box>
      )}

      {outputText ? (
        <Box sx={{ maxHeight: 500, overflow: 'auto' }}>
          <MarkdownPreview content={outputText} />
        </Box>
      ) : (
        <Typography variant="body2" sx={{ color: 'text.disabled', fontStyle: 'italic' }}>
          No output data
        </Typography>
      )}
    </Box>
  )
}

export { HistoricalRunSummary }
export type { HistoricalRunSummaryProps }
