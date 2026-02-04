import type { ReactNode } from 'react'
import { Box } from '@mui/material'

export type EditorToolbarProps = {
  children: ReactNode
  className?: string
}

export function EditorToolbar({ children, className }: EditorToolbarProps) {
  return (
    <Box
      className={className}
      sx={{
        borderBottom: 1,
        borderColor: 'divider',
        bgcolor: 'background.paper',
        px: 2,
        py: 1,
        display: 'flex',
        alignItems: 'center',
        gap: 1,
      }}
    >
      {children}
    </Box>
  )
}
