import { useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Chip from '@mui/material/Chip'
import IconButton from '@mui/material/IconButton'
import ExpandMoreIcon from '@mui/icons-material/ExpandMore'
import ExpandLessIcon from '@mui/icons-material/ExpandLess'
import { useStore } from '@/stores/lib'
import { boardStore } from '@/stores/boardStore'

/**
 * Collapsible Phase 0 summary for the dispatch panel.
 *
 * Reads directly from boardStore.lastResponse — no WebSocket needed.
 */
function PhaseZeroSummary() {
  const [expanded, setExpanded] = useState(false)
  const lastResponse = useStore(boardStore.store, boardStore.selectLastResponse)
  const status = useStore(boardStore.store, boardStore.selectStatus)

  if (status === 'idle' || status === 'submitting' || lastResponse === null) {
    return null
  }

  const p0 = lastResponse.phase_zero
  const snap = lastResponse.snapshot

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

  const totalChanges = p0.created_steps.length + p0.updated_steps.length + p0.deleted_steps.length
    + p0.created_edges.length + p0.deleted_edges.length + p0.rewired_edges.length + p0.moved_steps.length

  return (
    <Box sx={{ px: 1.5, py: 0.75 }}>
      <Box
        onClick={() => setExpanded((v) => !v)}
        sx={{ display: 'flex', alignItems: 'center', cursor: 'pointer', gap: 0.5, '&:hover': { opacity: 0.8 } }}
      >
        <IconButton size="small" sx={{ p: 0 }}>
          {expanded
            ? <ExpandLessIcon sx={{ fontSize: 14, color: 'text.secondary' }} />
            : <ExpandMoreIcon sx={{ fontSize: 14, color: 'text.secondary' }} />}
        </IconButton>
        <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary' }}>
          Phase 0
        </Typography>
        {lastResponse.is_first_submit && (
          <Chip label="first submit" size="small" color="info" variant="outlined" sx={{ height: 18, fontSize: 10 }} />
        )}
        <Typography variant="caption" sx={{ color: 'text.disabled', fontFamily: 'monospace', fontSize: 11, ml: 'auto' }}>
          {totalChanges > 0 ? `${totalChanges} change(s)` : 'no changes'}
        </Typography>
      </Box>

      {expanded && (
        <Box sx={{ pl: 3, mt: 0.5, display: 'flex', flexDirection: 'column', gap: 0.25 }}>
          <Typography variant="caption" sx={{ color: 'text.secondary', fontFamily: 'monospace', fontSize: 11 }}>
            {snap.nodes.length} node(s), {snap.edges.length} edge(s)
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
            <Typography variant="caption" sx={{ fontFamily: 'monospace', fontSize: 11, color: 'warning.main' }}>
              Dispatch score: {lastResponse.changeset.aggregate_score.toFixed(2)}
            </Typography>
          )}
          {lastResponse.changeset.noise.length > 0 && (
            <Typography variant="caption" sx={{ fontFamily: 'monospace', fontSize: 11, color: 'text.disabled' }}>
              Filtered: {lastResponse.changeset.noise.length} noise
            </Typography>
          )}
        </Box>
      )}
    </Box>
  )
}

export { PhaseZeroSummary }
