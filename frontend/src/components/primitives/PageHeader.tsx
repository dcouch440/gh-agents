import type { ReactNode } from 'react'
import { Box, Typography } from '@mui/material'

type PageHeaderProps = {
  title: string
  children?: ReactNode
}

function PageHeader({ title, children }: PageHeaderProps) {
  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        mb: 3,
      }}
    >
      <Typography variant="h4" component="h1" sx={{ fontWeight: 600 }}>
        {title}
      </Typography>
      {children ? (
        <Box sx={{ display: 'flex', gap: 2, alignItems: 'center' }}>
          {children}
        </Box>
      ) : null}
    </Box>
  )
}

export { PageHeader }
export type { PageHeaderProps }
