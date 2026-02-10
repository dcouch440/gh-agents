import type { ReactNode } from 'react'
import { Box, Typography } from '@mui/material'

type KeyValueProps = {
  label: string
  children: ReactNode
}

function KeyValue({ label, children }: KeyValueProps) {
  return (
    <Box sx={{ mb: 1 }}>
      <Typography variant="caption" color="text.secondary" component="div" sx={{ mb: 0.5 }}>
        {label}
      </Typography>
      <Typography variant="body2" component="div">
        {children}
      </Typography>
    </Box>
  )
}

export { KeyValue }
export type { KeyValueProps }
