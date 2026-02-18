import { useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import Tooltip from '@mui/material/Tooltip'
import CircularProgress from '@mui/material/CircularProgress'
import Chip from '@mui/material/Chip'
import Collapse from '@mui/material/Collapse'
import RefreshOutlined from '@mui/icons-material/RefreshOutlined'
import ExpandMoreOutlined from '@mui/icons-material/ExpandMoreOutlined'
import ExpandLessOutlined from '@mui/icons-material/ExpandLessOutlined'
import { useStepLastRun } from '@/hooks/useStepLastRun'
import { MetricChip } from '../../execution'
import { formatDuration, formatTokens, formatCost } from '../../formatters'
import type { PhaseExecution } from '@/types'

type LastRunTabProps = {
  stepId: string
}

const CHIP_STATUS_COLORS: Record<string, 'success' | 'error' | 'warning' | 'info' | 'default'> = {
  completed: 'success',
  complete: 'success',
  failed: 'error',
  running: 'warning',
  pending: 'info',
}

function PhaseCard({ phase }: { phase: PhaseExecution }) {
  const [open, setOpen] = useState(false)
  const hasContent = phase.output_content !== null && phase.output_content.length > 0

  const phaseLabel = phase.document_name
    ? `${phase.phase}: ${phase.document_name}`
    : phase.phase

  return (
    <Box sx={{ border: 1, borderColor: 'divider', borderRadius: 1, mb: 0.5 }}>
      <Box
        onClick={() => { if (hasContent) setOpen((o) => !o) }}
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 1,
          px: 1.5,
          py: 0.75,
          cursor: hasContent ? 'pointer' : 'default',
          '&:hover': hasContent ? { bgcolor: 'action.hover' } : undefined,
        }}
      >
        {hasContent && (
          open ? <ExpandLessOutlined sx={{ fontSize: 16 }} /> : <ExpandMoreOutlined sx={{ fontSize: 16 }} />
        )}
        <Typography variant="body2" sx={{ fontWeight: 500, flex: 1, textTransform: 'capitalize' }}>
          {phaseLabel}
        </Typography>
        <Chip
          label={phase.status}
          size="small"
          color={CHIP_STATUS_COLORS[phase.status] ?? 'default'}
          sx={{ height: 20, fontSize: '0.7rem' }}
        />
        {phase.input_tokens !== null && phase.output_tokens !== null && (
          <MetricChip label="tok" value={`${formatTokens(phase.input_tokens)}/${formatTokens(phase.output_tokens)}`} />
        )}
        {phase.cost_usd !== null && (
          <MetricChip label="" value={formatCost(phase.cost_usd)} />
        )}
      </Box>
      {hasContent && (
        <Collapse in={open}>
          <Box sx={{ px: 1.5, py: 1, borderTop: 1, borderColor: 'divider', maxHeight: 300, overflow: 'auto' }}>
            <Typography
              variant="body2"
              component="pre"
              sx={{ fontFamily: 'monospace', fontSize: '0.75rem', whiteSpace: 'pre-wrap', wordBreak: 'break-word', m: 0 }}
            >
              {phase.output_content}
            </Typography>
          </Box>
          {phase.error_message && (
            <Box sx={{ px: 1.5, py: 0.5, bgcolor: 'error.main', color: 'error.contrastText' }}>
              <Typography variant="caption">{phase.error_message}</Typography>
            </Box>
          )}
        </Collapse>
      )}
    </Box>
  )
}

function LastRunTab({ stepId }: LastRunTabProps) {
  const { data, isLoading, error, refresh } = useStepLastRun(stepId)

  if (isLoading && data === null) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <CircularProgress size={20} />
      </Box>
    )
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Header bar */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          px: 1.5,
          py: 0.5,
          borderBottom: 1,
          borderColor: 'divider',
        }}
      >
        <Typography variant="body2" sx={{ fontWeight: 500, color: 'text.secondary' }}>
          Last Run
        </Typography>
        <Tooltip title="Refresh">
          <span>
            <IconButton size="small" onClick={refresh} disabled={isLoading}>
              <RefreshOutlined fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
      </Box>

      {error ? (
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', p: 2 }}>
          <Typography variant="body2" color="error">{error}</Typography>
        </Box>
      ) : data === null ? (
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', p: 2 }}>
          <Typography variant="body2" color="text.secondary">
            No execution data yet. Run the workflow to see results.
          </Typography>
        </Box>
      ) : (
        <Box sx={{ flex: 1, overflow: 'auto', p: 1.5 }}>
          {/* Status + metrics row */}
          <Box sx={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: 1, mb: 1.5 }}>
            <Chip
              label={data.status}
              size="small"
              color={CHIP_STATUS_COLORS[data.status] ?? 'default'}
              sx={{ height: 22 }}
            />
            {data.duration_ms !== null && (
              <MetricChip label="Duration" value={formatDuration(data.duration_ms)} />
            )}
            {data.input_tokens !== null && data.output_tokens !== null && (
              <MetricChip label="Tokens" value={`${formatTokens(data.input_tokens)} in / ${formatTokens(data.output_tokens)} out`} />
            )}
            {data.cost_usd !== null && (
              <MetricChip label="Cost" value={formatCost(data.cost_usd)} />
            )}
          </Box>

          {/* Output section */}
          {data.output !== null && (
            <Box sx={{ mb: 1.5 }}>
              <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
                Output
              </Typography>
              <Box sx={{ bgcolor: 'action.hover', borderRadius: 1, p: 1, maxHeight: 200, overflow: 'auto' }}>
                <Typography
                  variant="body2"
                  component="pre"
                  sx={{ fontFamily: 'monospace', fontSize: '0.75rem', whiteSpace: 'pre-wrap', wordBreak: 'break-word', m: 0 }}
                >
                  {data.output}
                </Typography>
              </Box>
            </Box>
          )}

          {/* Phases section */}
          {data.phases !== null && data.phases.length > 0 && (
            <Box>
              <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
                Pipeline Phases ({data.phases.length})
              </Typography>
              {data.phases.map((phase) => (
                <PhaseCard key={phase.id} phase={phase} />
              ))}
            </Box>
          )}
        </Box>
      )}
    </Box>
  )
}

export { LastRunTab }
export type { LastRunTabProps }
