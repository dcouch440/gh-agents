import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Chip from '@mui/material/Chip'
import { useStore } from '@/stores/lib'
import { boardStore } from '@/stores/boardStore'

/**
 * Displays what Phase 0 did after a board submit.
 *
 * Reads directly from boardStore.lastResponse — no WebSocket needed.
 * This is the most immediately useful section because it's available
 * the instant the submit POST returns.
 */
function PhaseZeroSummary() {
  const lastResponse = useStore(boardStore.store, boardStore.selectLastResponse)
  const status = useStore(boardStore.store, boardStore.selectStatus)

  if (status === 'idle') {
    return (
      <Box sx={{ px: 0.5 }}>
        <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', mb: 0.5 }}>
          Phase 0
        </Typography>
        <Typography variant="caption" sx={{ color: 'text.disabled', fontFamily: 'monospace', fontSize: 11, display: 'block', mt: 0.5 }}>
          Submit the board to see results
        </Typography>
      </Box>
    )
  }

  if (status === 'submitting') {
    return (
      <Box sx={{ px: 0.5 }}>
        <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', mb: 0.5 }}>
          Phase 0
        </Typography>
        <Typography variant="caption" sx={{ color: 'text.disabled', fontFamily: 'monospace', fontSize: 11, display: 'block', mt: 0.5 }}>
          Submitting...
        </Typography>
      </Box>
    )
  }

  if (lastResponse === null) return null

  const p0 = lastResponse.phase_zero
  const snap = lastResponse.snapshot
  const hasDispatch = lastResponse.dispatch !== null

  const lines: string[] = []

  if (p0.created_steps.length > 0) {
    for (const step of p0.created_steps) {
      lines.push(`+ Step: "${step.name ?? step.id.slice(0, 8)}"`)
    }
  }
  if (p0.updated_steps.length > 0) {
    for (const step of p0.updated_steps) {
      lines.push(`~ Step: "${step.name ?? step.id.slice(0, 8)}"`)
    }
  }
  if (p0.created_edges.length > 0) {
    lines.push(`+ ${p0.created_edges.length} edge(s)`)
  }
  if (p0.deleted_steps.length > 0) {
    lines.push(`- ${p0.deleted_steps.length} step(s) deleted`)
  }
  if (p0.deleted_edges.length > 0) {
    lines.push(`- ${p0.deleted_edges.length} edge(s) deleted`)
  }
  if (p0.rewired_edges.length > 0) {
    lines.push(`~ ${p0.rewired_edges.length} edge(s) rewired`)
  }
  if (p0.moved_steps.length > 0) {
    lines.push(`~ ${p0.moved_steps.length} step(s) moved`)
  }

  if (snap.global_notes.length > 0) {
    for (const note of snap.global_notes) {
      lines.push(`Note: "${note.text.slice(0, 60)}"`)
    }
  }

  if (lines.length === 0) {
    lines.push('No changes detected')
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5, px: 0.5 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
        <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary' }}>
          Phase 0
        </Typography>
        {lastResponse.is_first_submit && (
          <Chip label="first submit" size="small" color="info" variant="outlined" sx={{ height: 18, fontSize: 10 }} />
        )}
        {hasDispatch && (
          <Chip label="dispatch" size="small" color="warning" variant="outlined" sx={{ height: 18, fontSize: 10 }} />
        )}
      </Box>

      <Typography variant="caption" sx={{ color: 'text.secondary', fontFamily: 'monospace', fontSize: 11 }}>
        Snapshot: {snap.nodes.length} node(s), {snap.edges.length} edge(s)
      </Typography>

      {lines.map((line, i) => (
        <Typography
          key={i}
          variant="caption"
          sx={{
            fontFamily: 'monospace',
            fontSize: 11,
            lineHeight: 1.5,
            color: line.startsWith('+') ? 'success.main' : line.startsWith('-') ? 'error.main' : 'text.secondary',
          }}
        >
          {line}
        </Typography>
      ))}

      {lastResponse.changeset.should_dispatch && (
        <Typography variant="caption" sx={{ fontFamily: 'monospace', fontSize: 11, color: 'warning.main', mt: 0.5 }}>
          Dispatch: score {lastResponse.changeset.aggregate_score.toFixed(2)}
        </Typography>
      )}
      {lastResponse.changeset.noise.length > 0 && (
        <Typography variant="caption" sx={{ fontFamily: 'monospace', fontSize: 11, color: 'text.disabled' }}>
          Filtered: {lastResponse.changeset.noise.length} noise change(s)
        </Typography>
      )}
    </Box>
  )
}

export { PhaseZeroSummary }
