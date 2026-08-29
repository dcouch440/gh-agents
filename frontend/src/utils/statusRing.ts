import { statusColor, designStatusColor } from '@/utils/statusColor'
import type { StatusPalette } from '@/theme'
import type { StepExecutionStatus } from '@/stores/workflowExecutionStore/types'
import type { SourceStreamStatus } from '@/stores/stepStreamStore'

/**
 * The resolved visual treatment for one node's outline.
 *
 * `null` means idle — the absence of a ring, not a transparent one. Callers
 * fall back to the plain `screenBorder` from `getNodeHighlightStyles`.
 */
type StatusRing = {
  readonly color: string
  /** Dashed rather than solid — a step the run stepped over. */
  readonly dashed: boolean
  /** Outer glow. Reserved for the two states worth interrupting someone for. */
  readonly glow: boolean
  /** Breathing animation. Suppressed at MINIMAL LOD by the caller. */
  readonly pulse: boolean
  /** Dims the node body along with the ring. */
  readonly dim: boolean
}

type ResolveStatusRingInput = {
  readonly status: StepExecutionStatus
  readonly designStatus: SourceStreamStatus | null
  readonly palette: StatusPalette
  /** False at MINIMAL LOD, where dozens of animated nodes would thrash. */
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
      dashed: status === 'skipped',
      dim: status === 'skipped',
      glow: status === 'running' || status === 'error',
      pulse: animated && status === 'running',
    }
  }

  const designColor = designStatus === null ? null : designStatusColor(designStatus, palette)
  if (designColor !== null) {
    return {
      color: designColor,
      dashed: false,
      dim: false,
      glow: designStatus === 'failed',
      pulse: animated && designStatus === 'running',
    }
  }

  return null
}

export { resolveStatusRing }
export type { StatusRing, ResolveStatusRingInput }
