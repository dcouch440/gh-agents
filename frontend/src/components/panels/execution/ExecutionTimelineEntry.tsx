import { useState, useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Collapse from '@mui/material/Collapse'
import { useTheme } from '@mui/material/styles'
import ExpandMoreOutlined from '@mui/icons-material/ExpandMoreOutlined'
import ExpandLessOutlined from '@mui/icons-material/ExpandLessOutlined'
import { StatusBadge } from '@/components/primitives'
import { statusColor } from '@/utils/statusColor'
import { ExecutionStepOutput } from './ExecutionStepOutput'
import { STATUS_LABELS, STATUS_BADGE_VARIANTS } from './constants'
import type { StepExecutionState } from '@/stores'

type ExecutionTimelineEntryProps = {
  stepId: string
  stepState: StepExecutionState
  isLast: boolean
}

const formatTokens = (input: number | null, output: number | null): string | null => {
  if (input === null && output === null) return null
  return `${input ?? 0} in / ${output ?? 0} out`
}

const formatDuration = (ms: number | null): string | null => {
  if (ms === null) return null
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function ExecutionTimelineEntry({ stepId, stepState, isLast }: ExecutionTimelineEntryProps) {
  const theme = useTheme()
  const [expanded, setExpanded] = useState(false)
  const toggle = useCallback(() => setExpanded((prev) => !prev), [])

  const { status, stepName, output, error, inputTokens, outputTokens, durationMs, forEachProgress } = stepState
  // Idle has no status color by design; the timeline still needs an ink for its
  // rail, so it borrows the neutral `pending` grey.
  const color = statusColor(status, theme.palette.statusPalette) ?? theme.palette.statusPalette.pending
  const hasOutput = output !== null || error !== null
  const isRunning = status === 'running'

  const tokens = formatTokens(inputTokens, outputTokens)
  const duration = formatDuration(durationMs)
  const forEach = forEachProgress ? `${forEachProgress.completed}/${forEachProgress.total}` : null

  return (
    <Box sx={{ display: 'flex', minHeight: 48 }}>
      {/* Timeline gutter */}
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          width: 24,
          flexShrink: 0,
          pt: 1.5,
        }}
      >
        <Box
          sx={{
            width: 10,
            height: 10,
            borderRadius: '50%',
            backgroundColor: color,
            flexShrink: 0,
            ...(isRunning && {
              '@keyframes timelinePulse': {
                '0%, 100%': { opacity: 1, transform: 'scale(1)' },
                '50%': { opacity: 0.5, transform: 'scale(1.3)' },
              },
              animation: 'timelinePulse 1.5s ease-in-out infinite',
            }),
          }}
        />
        {!isLast && (
          <Box
            sx={{
              flex: 1,
              width: 2,
              backgroundColor: 'divider',
              mt: 0.5,
            }}
          />
        )}
      </Box>

      {/* Content */}
      <Box sx={{ flex: 1, minWidth: 0, pb: isLast ? 0 : 1 }}>
        <Box
          onClick={hasOutput ? toggle : undefined}
          sx={{
            display: 'flex',
            alignItems: 'center',
            gap: 1,
            py: 0.75,
            px: 1,
            borderRadius: 1,
            cursor: hasOutput ? 'pointer' : 'default',
            '&:hover': hasOutput ? { backgroundColor: 'action.hover' } : {},
          }}
        >
          <Typography
            variant="body2"
            sx={{
              fontWeight: 500,
              flex: 1,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {stepName ?? stepId}
          </Typography>
          <StatusBadge label={STATUS_LABELS[status]} variant={STATUS_BADGE_VARIANTS[status]} />
          {hasOutput && (
            <Box sx={{ color: 'text.secondary', display: 'flex', alignItems: 'center' }}>
              {expanded ? <ExpandLessOutlined fontSize="small" /> : <ExpandMoreOutlined fontSize="small" />}
            </Box>
          )}
        </Box>

        {/* Metrics row */}
        {(duration ?? tokens ?? forEach) !== null && (
          <Box sx={{ display: 'flex', gap: 1.5, px: 1, pb: 0.5, flexWrap: 'wrap' }}>
            {duration && (
              <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                {duration}
              </Typography>
            )}
            {tokens && (
              <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                {tokens}
              </Typography>
            )}
            {forEach && (
              <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                {forEach} items
              </Typography>
            )}
          </Box>
        )}

        {/* Expanded output */}
        <Collapse in={expanded} timeout={150}>
          {expanded && <ExecutionStepOutput output={output} error={error} />}
        </Collapse>
      </Box>
    </Box>
  )
}

export { ExecutionTimelineEntry }
export type { ExecutionTimelineEntryProps }
