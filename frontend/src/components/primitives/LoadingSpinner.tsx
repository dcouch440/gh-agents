import { Box, CircularProgress } from '@mui/material'

type SpinnerSize = 'sm' | 'md' | 'lg'

type LoadingSpinnerProps = {
  size?: SpinnerSize
  centered?: boolean
}

const SIZE_MAP = {
  sm: 20,
  md: 40,
  lg: 60,
} as const

function LoadingSpinner({ size = 'md', centered = false }: LoadingSpinnerProps) {
  const spinner = <CircularProgress size={SIZE_MAP[size]} />

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
