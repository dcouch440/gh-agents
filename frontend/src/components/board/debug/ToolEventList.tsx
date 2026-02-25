import Box from '@mui/material/Box'
import { ToolEvent } from './ToolEvent'
import type { DispatchTraceEvent } from '@/stores/dispatchStore'

type ToolEventListProps = {
  readonly trace: readonly DispatchTraceEvent[]
}

/**
 * Renders tool_start, tool_end, and error events from a dispatch trace.
 * Filters out token events (those are rendered by TokenStream).
 */
function ToolEventList({ trace }: ToolEventListProps) {
  const toolEvents = trace.filter(
    (e): e is DispatchTraceEvent & { type: 'tool_start' | 'tool_end' | 'error' } =>
      e.type === 'tool_start' || e.type === 'tool_end' || e.type === 'error',
  )

  if (toolEvents.length === 0) return null

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column' }}>
      {toolEvents.map((event, i) => (
        <ToolEvent key={`${event.ts}-${i}`} event={event} />
      ))}
    </Box>
  )
}

export { ToolEventList }
export type { ToolEventListProps }
