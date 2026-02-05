import type { ReactNode } from 'react'
import { Box, Typography } from '@mui/material'

type EmptyStateProps = {
  icon?: ReactNode
  message: string
  action?: ReactNode
}

function EmptyState({ icon, message, action }: EmptyStateProps) {
  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        p: 4,
        textAlign: 'center',
        gap: 1.5,
      }}
    >
      {icon ? (
        <Box sx={{ color: 'text.secondary', opacity: 0.5, fontSize: '2.5rem', display: 'flex' }}>
          {icon}
        </Box>
      ) : null}
      <Typography variant="body2" color="text.secondary">
        {message}
      </Typography>
      {action ? <Box sx={{ mt: 1 }}>{action}</Box> : null}
    </Box>
  )
}

export { EmptyState }
export type { EmptyStateProps }
