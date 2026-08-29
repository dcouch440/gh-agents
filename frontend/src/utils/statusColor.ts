import type { StatusPalette } from '@/theme'

/**
 * One status → color mapping, shared by the canvas ring, the node badge, the
 * sidebar dot and the execution panel.
 *
 * Keyed by `string` rather than a union on purpose. Three vocabularies are in
 * play for the same states — the API says `completed`/`failed`, the execution
 * store says `success`/`error`, and protocol phases say `complete` — because
 * backend statuses are untyped `text` columns rather than a Rust enum. Taking
 * the string and normalizing here is honest about that, and means a caller
 * cannot pick the wrong map for its vocabulary.
 *
 * Returns `null` for idle and for anything unrecognized: no color is the
 * correct answer for "nothing is happening", and an unknown status should stay
 * silent rather than guess.
 */
const statusColor = (status: string, palette: StatusPalette): string | null => {
  switch (status) {
    case 'running':
      return palette.running
    case 'success':
    case 'completed':
    case 'complete':
      return palette.finished
    case 'error':
    case 'failed':
      return palette.failed
    case 'paused':
      return palette.paused
    case 'skipped':
      return palette.skipped
    case 'pending':
      return palette.pending
    default:
      return null
  }
}

/**
 * Design-phase color — the second axis, held apart from the run colors.
 *
 * A node can be mid-design while its run status is still idle, so this never
 * competes with `statusColor`; callers fall back to it only when the run axis
 * has nothing to say. `failed` is the one state the two axes share.
 */
const designStatusColor = (status: string, palette: StatusPalette): string | null => {
  switch (status) {
    case 'running':
      return palette.designing
    case 'completed':
      return palette.designed
    case 'failed':
      return palette.failed
    default:
      return null
  }
}

export { statusColor, designStatusColor }
