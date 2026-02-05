import type { ReactNode } from 'react'
import { Paper, Box, Typography } from '@mui/material'
import { ANIMATION } from '@/constants'

type CardProps = {
  title?: string
  actions?: ReactNode
  children: ReactNode
  hoverable?: boolean
}

function Card({ title, actions, children, hoverable }: CardProps) {
  return (
    <Paper
      elevation={0}
      sx={{
        border: 1,
        borderColor: 'divider',
        borderRadius: 2,
        overflow: 'hidden',
        ...(hoverable
          ? {
              transition: `transform ${ANIMATION.FAST}ms ease, box-shadow ${ANIMATION.FAST}ms ease`,
              '&:hover': {
                transform: 'translateY(-2px)',
                boxShadow: 2,
              },
            }
          : {}),
      }}
    >
      {title ? (
        <Box
          sx={{
            p: 2,
            borderBottom: 1,
            borderColor: 'divider',
            bgcolor: 'background.paper',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <Typography variant="h6" component="h3" sx={{ fontWeight: 600 }}>
            {title}
          </Typography>
          {actions ? <Box>{actions}</Box> : null}
        </Box>
      ) : null}
      <Box sx={{ p: 2 }}>{children}</Box>
    </Paper>
  )
}

export { Card }
export type { CardProps }
