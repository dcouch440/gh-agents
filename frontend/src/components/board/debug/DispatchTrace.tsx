import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Chip from '@mui/material/Chip'
import { useStore } from '@/stores/lib'
import { boardStore } from '@/stores/boardStore'
import { dispatchStore } from '@/stores/dispatchStore'
import type { DispatchEntry } from '@/stores/dispatchStore'
import { useDispatchPoll } from '../hooks'
import { statusColor } from './utils'
import { TokenStream } from './TokenStream'
import { ToolEventList } from './ToolEventList'

/**
 * Dispatch trace section: shows the board dispatcher's streaming progress.
 *
 * Reads the dispatch step ID from the last board submit response,
 * then shows the trace from dispatchStore (fed by WS + REST polling).
 */
function DispatchTrace() {
  const lastResponse = useStore(boardStore.store, boardStore.selectLastResponse)
  const dispatches = lastResponse?.dispatches ?? []
  const dispatch = dispatches.length > 0 ? dispatches[0] ?? null : null
  const stepId = dispatch?.step_id ?? null

  const displayEntry: DispatchEntry | null = useStore(
    dispatchStore.store,
    stepId !== null ? dispatchStore.selectByStepId(stepId) : () => null,
  )

  // Poll REST trace endpoint while dispatch is running
  useDispatchPoll(dispatch?.execution_id ?? null)

  if (displayEntry === null) {
    return (
      <Box sx={{ px: 0.5 }}>
        <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', mb: 0.5 }}>
          Dispatch Trace
        </Typography>
        <Typography variant="caption" sx={{ color: 'text.disabled', fontFamily: 'monospace', fontSize: 11, display: 'block', mt: 0.5 }}>
          {dispatch === null ? 'No dispatch triggered' : 'Waiting for events...'}
        </Typography>
      </Box>
    )
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1, px: 0.5 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
        <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary' }}>
          Dispatch Trace
        </Typography>
        <Chip
          label={displayEntry.status}
          size="small"
          color={statusColor(displayEntry.status)}
          variant="outlined"
          sx={{ height: 18, fontSize: 10 }}
        />
      </Box>

      {displayEntry.instruction.length > 0 && (
        <Typography
          variant="caption"
          sx={{ fontFamily: 'monospace', fontSize: 11, color: 'text.disabled', fontStyle: 'italic' }}
        >
          {displayEntry.instruction.slice(0, 120)}
          {displayEntry.instruction.length > 120 ? '...' : ''}
        </Typography>
      )}

      <TokenStream text={displayEntry.tokenBuffer} />
      <ToolEventList trace={displayEntry.trace} />

      {displayEntry.summary !== null && (
        <Typography variant="caption" sx={{ fontFamily: 'monospace', fontSize: 11, color: 'success.main' }}>
          {displayEntry.summary}
        </Typography>
      )}

      {displayEntry.error !== null && (
        <Typography variant="caption" sx={{ color: 'error.main', fontFamily: 'monospace', fontSize: 11 }}>
          {displayEntry.error}
        </Typography>
      )}
    </Box>
  )
}

export { DispatchTrace }
