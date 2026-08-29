import { statusColor, designStatusColor } from '@/utils/statusColor'
import type { StatusPalette } from '@/theme'
import type { StepExecutionStatus } from '@/stores/workflowExecutionStore/types'
import type { SourceStreamStatus } from '@/stores/stepStreamStore'

/**
 * The resolved visual treatment for one node's outline.
 *
 * `null` means the node has no status yet — nothing has been designed and
 * nothing has run. That is a state the board draws (a dashed outline), but it
 * is not a *status*, so it is represented by the absence of a ring rather than
 * by a ring that means "nothing". The board's `resolveBoxStroke` owns what an
 * absent ring looks like.
 *
 * There is deliberately no `dashed` field. Dash carries exactly one meaning on
 * this canvas — "not designed yet" — and that is the null case, so no status
 * may claim it.
 */
type StatusRing = {
  readonly color: string
  /** Outer glow. Reserved for the two states worth interrupting someone for. */
  readonly glow: boolean
  /** Breathing animation. Suppressed below `BOARD_RING.ANIMATE_MIN_ZOOM`. */
  readonly pulse: boolean
  /** Dims the node body along with the ring — the run stepped over it. */
  readonly dim: boolean
}

type ResolveStatusRingInput = {
  readonly status: StepExecutionStatus
  readonly designStatus: SourceStreamStatus | null
  readonly palette: StatusPalette
  /** False when zoomed out, where dozens of animated nodes would thrash. */
  readonly animated: boolean
}

/**
 * Collapse the run axis and the design axis into a single ring.
 *
 * The two are genuinely independent — a node can be mid-design while its run
 * status is still `idle` — so the run axis is asked first and the design axis
 * only speaks when the run has nothing to say. That ordering is what keeps a
 * node that failed a run reading as failed while it is being redesigned.
 *
 * Colors come from the shared `statusColor`, so the ring, the node badge and
 * the sidebar dot cannot drift apart; only the treatment is decided here.
 */
const resolveStatusRing = ({
  status,
  designStatus,
  palette,
  animated,
}: ResolveStatusRingInput): StatusRing | null => {
  const runColor = statusColor(status, palette)
  if (runColor !== null) {
    return {
      color: runColor,
      // A skipped step recedes rather than competing: the run passed it over,
      // which is worth showing but never worth reading first.
      dim: status === 'skipped',
      glow: status === 'running' || status === 'error',
      pulse: animated && status === 'running',
    }
  }

  const designColor = designStatus === null ? null : designStatusColor(designStatus, palette)
  if (designColor !== null) {
    return {
      color: designColor,
      dim: false,
      glow: designStatus === 'failed',
      pulse: animated && designStatus === 'running',
    }
  }

  return null
}

export { resolveStatusRing }
export type { StatusRing, ResolveStatusRingInput }
