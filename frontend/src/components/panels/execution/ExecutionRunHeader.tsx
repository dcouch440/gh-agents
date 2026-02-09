import { useState, useEffect, useRef } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

type ExecutionRunHeaderProps = {
  isRunning: boolean
  completedSteps: number
  totalSteps: number
  durationMs: number | null
  error: string | null
  startedAt: string | null
  completedAt: string | null
}

const formatDuration = (ms: number): string => {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`
  const mins = Math.floor(ms / 60_000)
  const secs = Math.round((ms % 60_000) / 1000)
  return `${mins}m ${secs}s`
}

function ExecutionRunHeader({
  isRunning,
  completedSteps,
  totalSteps,
  durationMs,
  error,
  startedAt,
  completedAt,
}: ExecutionRunHeaderProps) {
  const [elapsed, setElapsed] = useState<number>(0)
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  useEffect(() => {
    if (!isRunning || !startedAt) {
      if (intervalRef.current) clearInterval(intervalRef.current)
      return undefined
    }
    const start = new Date(startedAt).getTime()
    const tick = () => setElapsed(Date.now() - start)
    tick()
    intervalRef.current = setInterval(tick, 250)
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current)
    }
  }, [isRunning, startedAt])

  const displayDuration = isRunning ? elapsed : durationMs
  const isFailed = !isRunning && error !== null
  const isCompleted = !isRunning && completedAt !== null && error === null

  const statusColor = isRunning ? '#3b82f6' : isFailed ? '#f85149' : isCompleted ? '#2dd4bf' : '#7d8590'
  const statusLabel = isRunning ? 'Running...' : isFailed ? 'Failed' : isCompleted ? 'Completed' : 'Idle'

  return (
    <Box sx={{ px: 2, py: 1.5, borderBottom: 1, borderColor: 'divider' }}>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 0.5 }}>
        <Box
          sx={{
            width: 8,
            height: 8,
            borderRadius: '50%',
            backgroundColor: statusColor,
            flexShrink: 0,
            ...(isRunning && {
              '@keyframes pulse': {
                '0%, 100%': { opacity: 1 },
                '50%': { opacity: 0.4 },
              },
              animation: 'pulse 1.5s ease-in-out infinite',
            }),
          }}
        />
        <Typography variant="body2" sx={{ fontWeight: 600, color: statusColor }}>
          {statusLabel}
        </Typography>
        <Typography variant="body2" sx={{ color: 'text.secondary', ml: 'auto' }}>
          {completedSteps} / {totalSteps} steps
        </Typography>
      </Box>
      {displayDuration !== null && displayDuration > 0 && (
        <Typography variant="caption" sx={{ color: 'text.secondary' }}>
          {formatDuration(displayDuration)}
        </Typography>
      )}
      {isFailed && error && (
        <Typography
          variant="caption"
          sx={{
            display: 'block',
            mt: 0.5,
            color: '#f85149',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {error}
        </Typography>
      )}
    </Box>
  )
}

export { ExecutionRunHeader }
export type { ExecutionRunHeaderProps }
