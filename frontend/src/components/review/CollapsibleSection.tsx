import type { ReactNode } from 'react'
import { Box, Collapse, Typography } from '@mui/material'

type CollapsibleSectionProps = {
  title: string
  open: boolean
  onToggle: () => void
  children: ReactNode
}

function CollapsibleSection({ title, open, onToggle, children }: CollapsibleSectionProps) {
  return (
    <Box sx={{ mb: 1 }}>
      <Box
        onClick={onToggle}
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 1,
          cursor: 'pointer',
          py: 0.5,
          px: 1,
          borderRadius: 1,
          '&:hover': { bgcolor: 'action.hover' },
          userSelect: 'none',
        }}
      >
        <Typography
          variant="caption"
          sx={{ fontFamily: 'monospace', color: 'text.secondary', lineHeight: 1 }}
        >
          {open ? '\u25BC' : '\u25B6'}
        </Typography>
        <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
          {title}
        </Typography>
      </Box>
      <Collapse in={open}>
        <Box sx={{ px: 1, pt: 0.5 }}>{children}</Box>
      </Collapse>
    </Box>
  )
}

export { CollapsibleSection }
export type { CollapsibleSectionProps }
