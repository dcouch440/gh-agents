import Box from '@mui/material/Box'
import type { StepExecutionStatus } from '@/stores/workflowExecutionStore/types'

type StatusDotProps = {
  readonly status: StepExecutionStatus | undefined
}

const SIZE = 6

function StatusDot({ status }: StatusDotProps) {
  const resolved = status ?? 'idle'

  if (resolved === 'running') {
    return (
      <Box
        sx={{
          width: SIZE,
          height: SIZE,
          borderRadius: '50%',
          flexShrink: 0,
          background: `conic-gradient(#e3b341 0deg, #e3b341 180deg, transparent 180deg, transparent 360deg)`,
          animation: 'statusDotSpin 1s linear infinite',
          '@keyframes statusDotSpin': {
            from: { transform: 'rotate(0deg)' },
            to: { transform: 'rotate(360deg)' },
          },
        }}
      />
    )
  }

  const filled = resolved === 'success' || resolved === 'error'
  const color =
    resolved === 'success' ? '#3fb950' :
    resolved === 'error' ? '#f85149' :
    undefined

  return (
    <Box
      sx={{
        width: SIZE,
        height: SIZE,
        borderRadius: '50%',
        flexShrink: 0,
        ...(filled
          ? { backgroundColor: color }
          : { border: '1px solid', borderColor: 'text.disabled' }),
      }}
    />
  )
}

export { StatusDot }
export type { StatusDotProps }
