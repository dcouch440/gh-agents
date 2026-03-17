import Box from '@mui/material/Box'
import type { StepExecutionStatus } from '@/stores/workflowExecutionStore/types'
import type { SourceStreamStatus } from '@/stores/stepStreamStore'

type StatusDotProps = {
  readonly status: StepExecutionStatus | undefined
  readonly designStatus?: SourceStreamStatus | null
}

const SIZE = 6

function StatusDot({ status, designStatus }: StatusDotProps) {
  const resolved = status ?? 'idle'

  // When execution is running and design status exists, show design indicator
  // instead of generic yellow — workforce steps show design phase, agent dots handle execution
  if (resolved === 'running' && designStatus !== null) {
    const color = designStatus === 'failed' ? '#f85149' : '#58a6ff'
    if (designStatus === 'running') {
      return (
        <Box
          sx={{
            width: SIZE,
            height: SIZE,
            borderRadius: '50%',
            flexShrink: 0,
            background: `conic-gradient(${color} 0deg, ${color} 180deg, transparent 180deg, transparent 360deg)`,
            animation: 'statusDotSpin 1s linear infinite',
            '@keyframes statusDotSpin': {
              from: { transform: 'rotate(0deg)' },
              to: { transform: 'rotate(360deg)' },
            },
          }}
        />
      )
    }
    return (
      <Box
        sx={{
          width: SIZE,
          height: SIZE,
          borderRadius: '50%',
          flexShrink: 0,
          backgroundColor: color,
        }}
      />
    )
  }

  // Execution status takes precedence when active (non-workforce steps)
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

  if (resolved === 'success' || resolved === 'error') {
    const color = resolved === 'success' ? '#3fb950' : '#f85149'
    return (
      <Box
        sx={{
          width: SIZE,
          height: SIZE,
          borderRadius: '50%',
          flexShrink: 0,
          backgroundColor: color,
        }}
      />
    )
  }

  // When idle, show design status if present
  if (designStatus === 'running') {
    return (
      <Box
        sx={{
          width: SIZE,
          height: SIZE,
          borderRadius: '50%',
          flexShrink: 0,
          background: `conic-gradient(#58a6ff 0deg, #58a6ff 180deg, transparent 180deg, transparent 360deg)`,
          animation: 'statusDotSpin 1s linear infinite',
          '@keyframes statusDotSpin': {
            from: { transform: 'rotate(0deg)' },
            to: { transform: 'rotate(360deg)' },
          },
        }}
      />
    )
  }

  if (designStatus === 'completed') {
    return (
      <Box
        sx={{
          width: SIZE,
          height: SIZE,
          borderRadius: '50%',
          flexShrink: 0,
          backgroundColor: '#58a6ff',
        }}
      />
    )
  }

  if (designStatus === 'failed') {
    return (
      <Box
        sx={{
          width: SIZE,
          height: SIZE,
          borderRadius: '50%',
          flexShrink: 0,
          backgroundColor: '#f85149',
        }}
      />
    )
  }

  // Default: idle, no design status
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

export { StatusDot }
export type { StatusDotProps }
