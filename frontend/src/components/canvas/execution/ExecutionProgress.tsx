import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import LinearProgress from '@mui/material/LinearProgress'

type ExecutionProgressProps = {
  completed: number
  total: number
  label?: string
  accentColor?: string
}

function ExecutionProgress({
  completed,
  total,
  label = '',
  accentColor = '#3b82f6',
}: ExecutionProgressProps) {
  const value = total === 0 ? 0 : (completed / total) * 100
  const displayLabel =
    label === '' ? `${completed}/${total}` : `${completed}/${total} ${label}`

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.25 }}>
      <LinearProgress
        variant="determinate"
        value={value}
        sx={{
          height: 3,
          borderRadius: 2,
          backgroundColor: `${accentColor}33`,
          '& .MuiLinearProgress-bar': {
            backgroundColor: accentColor,
            borderRadius: 2,
          },
        }}
      />
      <Typography sx={{ fontSize: 10, color: 'text.secondary' }}>
        {displayLabel}
      </Typography>
    </Box>
  )
}

export { ExecutionProgress }
export type { ExecutionProgressProps }
