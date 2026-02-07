// ============================================================================
// bridge/nodeHelpers — Shared helpers for custom node components
// ============================================================================

import type { SxProps, Theme } from '@mui/material'
import type { StepNodeData } from './types'

// ── Status Color ─────────────────────────────────────────────────────────────

const STATUS_COLORS: Record<string, string> = {
  idle: 'grey.400',
  pending: 'info.main',
  running: 'primary.main',
  success: 'success.main',
  error: 'error.main',
  skipped: 'grey.500',
  paused: 'warning.main',
}

const getStatusColor = (status: string | undefined): string =>
  (status && STATUS_COLORS[status]) ?? 'grey.400'

// ── Shared Node Box Style ────────────────────────────────────────────────────

const nodeBoxSx = (data: StepNodeData, selected: boolean, borderColor: string): SxProps<Theme> => ({
  width: 200,
  minHeight: 72,
  border: 2,
  borderColor: selected ? 'primary.main' : borderColor,
  borderRadius: 2,
  bgcolor: data.hovered ? 'action.hover' : 'background.paper',
  p: 1.5,
  boxShadow: selected ? 3 : 1,
  cursor: 'grab',
})

export { getStatusColor, nodeBoxSx }
