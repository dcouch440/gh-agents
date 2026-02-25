import Typography from '@mui/material/Typography'
import Box from '@mui/material/Box'
import BuildIcon from '@mui/icons-material/Build'
import CheckCircleOutlineIcon from '@mui/icons-material/CheckCircleOutline'
import ErrorOutlineIcon from '@mui/icons-material/ErrorOutline'
import type { DispatchTraceEvent } from '@/stores/dispatchStore'

type ToolEventProps = {
  readonly event: DispatchTraceEvent & { type: 'tool_start' | 'tool_end' | 'error' }
}

/**
 * Single tool call event: icon + tool name + status indicator.
 */
function ToolEvent({ event }: ToolEventProps) {
  if (event.type === 'error') {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, py: 0.25 }}>
        <ErrorOutlineIcon sx={{ fontSize: 14, color: 'error.main' }} />
        <Typography variant="caption" sx={{ fontFamily: 'monospace', fontSize: 11, color: 'error.main' }}>
          {event.error}
        </Typography>
      </Box>
    )
  }

  const isDone = event.type === 'tool_end'
  const icon = isDone
    ? <CheckCircleOutlineIcon sx={{ fontSize: 14, color: 'success.main' }} />
    : <BuildIcon sx={{ fontSize: 14, color: 'text.disabled' }} />

  return (
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, py: 0.25 }}>
      {icon}
      <Typography variant="caption" sx={{ fontFamily: 'monospace', fontSize: 11, color: 'text.secondary' }}>
        {event.toolName}
      </Typography>
      {!isDone && (
        <Typography variant="caption" sx={{ fontFamily: 'monospace', fontSize: 10, color: 'text.disabled' }}>
          running...
        </Typography>
      )}
    </Box>
  )
}

export { ToolEvent }
export type { ToolEventProps }
