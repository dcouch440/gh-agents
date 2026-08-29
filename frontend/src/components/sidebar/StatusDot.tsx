import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { statusColor, designStatusColor } from '@/utils/statusColor'
import type { StepExecutionStatus } from '@/stores/workflowExecutionStore/types'
import type { SourceStreamStatus } from '@/stores/stepStreamStore'

type StatusDotProps = {
  readonly status: StepExecutionStatus | undefined
  readonly designStatus?: SourceStreamStatus | null
  /** Pinned-completed is drawn as a ring so it reads apart from a fresh result. */
  readonly pinned?: boolean
}

const SIZE = 6

/**
 * The sidebar's per-step dot.
 *
 * Colors come from the theme's status palette via the shared `statusColor`, so
 * the dot and the node's ring on the canvas are the same color for the same
 * state by construction rather than by two lists agreeing.
 */
function StatusDot({ status, designStatus, pinned }: StatusDotProps) {
  const theme = useTheme()
  const palette = theme.palette.statusPalette
  const resolved = status ?? 'idle'

  // An active design phase describes a step better than a generic "running":
  // workforce steps design their agents before any of them execute. The design
  // axis only gets to speak while the run axis is running or silent.
  const design = designStatus !== null && designStatus !== undefined && designStatus !== 'idle'
    ? designStatus
    : null
  const showDesign = design !== null && (resolved === 'running' || resolved === 'idle')

  const color = showDesign ? designStatusColor(design, palette) : statusColor(resolved, palette)

  if (color === null) {
    return (
      <Box
        sx={{
          width: SIZE,
          height: SIZE,
          borderRadius: '50%',
          flexShrink: 0,
          border: '1px solid',
          borderColor: 'text.disabled',
        }}
      />
    )
  }

  const spinning = showDesign ? design === 'running' : resolved === 'running'

  return (
    <Box
      sx={{
        width: SIZE,
        height: SIZE,
        borderRadius: '50%',
        flexShrink: 0,
        ...(spinning
          ? {
              background: `conic-gradient(${color} 0deg, ${color} 180deg, transparent 180deg, transparent 360deg)`,
              animation: 'statusDotSpin 1s linear infinite',
              '@media (prefers-reduced-motion: reduce)': { animation: 'none' },
            }
          : { backgroundColor: color }),
        ...(pinned === true && resolved === 'success'
          ? { boxShadow: `0 0 0 2px ${color}47` }
          : {}),
      }}
    />
  )
}

export { StatusDot }
export type { StatusDotProps }
