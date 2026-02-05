import { Box, CircularProgress, Typography } from '@mui/material'

type SpinnerSize = 'sm' | 'md' | 'lg'

type LoadingSpinnerProps = {
  size?: SpinnerSize
  centered?: boolean
  label?: string
}

const SIZE_MAP = {
  sm: 20,
  md: 40,
  lg: 60,
} as const

function LoadingSpinner({ size = 'md', centered = false, label }: LoadingSpinnerProps) {
  const spinner = (
    <Box sx={{ display: 'inline-flex', flexDirection: 'column', alignItems: 'center', gap: 1.5 }}>
      <CircularProgress size={SIZE_MAP[size]} color="primary" />
      {label ? (
        <Typography variant="caption" color="text.secondary">
          {label}
        </Typography>
      ) : null}
    </Box>
  )

  if (centered) {
    return (
      <Box
        sx={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          p: 2,
        }}
      >
        {spinner}
      </Box>
    )
  }

  return spinner
}

export { LoadingSpinner }
export type { SpinnerSize, LoadingSpinnerProps }
