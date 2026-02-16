import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { StatusBadge } from '@/components/primitives'
import { STATUS_COLORS, STATUS_LABELS, STATUS_BADGE_VARIANTS } from './constants'
import type { SubWorkflowProgress, ChildStepState, StepExecutionStatus } from '@/stores'

type ChildStepTimelineProps = {
  progress: SubWorkflowProgress
}

const mapChildStatus = (status: ChildStepState['status']): StepExecutionStatus => {
  if (status === 'success') return 'success'
  if (status === 'error') return 'error'
  return 'running'
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

function ChildStepTimeline({ progress }: ChildStepTimelineProps) {
  const { childSteps, completedSteps, totalSteps, status: overallStatus } = progress

  return (
    <Box sx={{ ml: 3, mt: 0.5, mb: 1, pl: 1.5, borderLeft: '2px solid', borderColor: 'divider' }}>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 0.5 }}>
        <Typography variant="caption" sx={{ color: 'text.secondary', fontWeight: 600 }}>
          Sub-workflow
        </Typography>
        <Typography variant="caption" sx={{ color: 'text.secondary' }}>
          {completedSteps}/{totalSteps} steps
        </Typography>
        {overallStatus === 'failed' && (
          <StatusBadge label="Failed" variant="error" />
        )}
      </Box>

      {childSteps.map((child) => {
        const stepStatus = mapChildStatus(child.status)
        const color = STATUS_COLORS[stepStatus]
        const isRunning = child.status === 'running'
        const tokens = formatTokens(child.inputTokens, child.outputTokens)
        const duration = formatDuration(child.durationMs)

        return (
          <Box key={child.childStepId} sx={{ display: 'flex', alignItems: 'flex-start', minHeight: 32, mb: 0.25 }}>
            <Box
              sx={{
                width: 8,
                height: 8,
                borderRadius: '50%',
                backgroundColor: color,
                flexShrink: 0,
                mt: 0.75,
                mr: 1,
                ...(isRunning && {
                  '@keyframes childPulse': {
                    '0%, 100%': { opacity: 1, transform: 'scale(1)' },
                    '50%': { opacity: 0.5, transform: 'scale(1.3)' },
                  },
                  animation: 'childPulse 1.5s ease-in-out infinite',
                }),
              }}
            />

            <Box sx={{ flex: 1, minWidth: 0 }}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.75 }}>
                <Typography
                  variant="caption"
                  sx={{
                    fontWeight: 500,
                    flex: 1,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {child.childStepName}
                </Typography>
                <StatusBadge
                  label={STATUS_LABELS[stepStatus]}
                  variant={STATUS_BADGE_VARIANTS[stepStatus]}
                />
              </Box>

              {(duration !== null || tokens !== null) && (
                <Box sx={{ display: 'flex', gap: 1, mt: 0.25 }}>
                  {duration !== null && (
                    <Typography variant="caption" sx={{ color: 'text.secondary', fontSize: '0.7rem' }}>
                      {duration}
                    </Typography>
                  )}
                  {tokens !== null && (
                    <Typography variant="caption" sx={{ color: 'text.secondary', fontSize: '0.7rem' }}>
                      {tokens}
                    </Typography>
                  )}
                </Box>
              )}

              {child.error !== null && (
                <Typography
                  variant="caption"
                  sx={{
                    color: '#f85149',
                    fontFamily: 'monospace',
                    fontSize: '0.7rem',
                    display: 'block',
                    mt: 0.25,
                  }}
                >
                  {child.error}
                </Typography>
              )}
            </Box>
          </Box>
        )
      })}
    </Box>
  )
}

export { ChildStepTimeline }
export type { ChildStepTimelineProps }
