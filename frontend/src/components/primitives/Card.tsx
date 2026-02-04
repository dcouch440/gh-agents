import type { ReactNode } from 'react'
import { Paper, Box, Typography } from '@mui/material'

type CardProps = {
  title?: string
  children: ReactNode
}

function Card({ title, children }: CardProps) {
  return (
    <Paper
      elevation={0}
      sx={{
        border: 1,
        borderColor: 'divider',
        borderRadius: 2,
        overflow: 'hidden',
      }}
    >
      {title ? (
        <Box
          sx={{
            p: 2,
            borderBottom: 1,
            borderColor: 'divider',
            bgcolor: 'background.paper',
          }}
        >
          <Typography variant="h6" component="h3" sx={{ fontWeight: 600 }}>
            {title}
          </Typography>
        </Box>
      ) : null}
      <Box sx={{ p: 2 }}>{children}</Box>
    </Paper>
  )
}

export { Card }
export type { CardProps }
