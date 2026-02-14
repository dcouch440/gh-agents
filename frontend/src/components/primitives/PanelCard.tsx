import type { ReactNode } from 'react'
import { Paper, Box, Typography } from '@mui/material'

type PanelCardProps = {
  depth: number
  title: string
  children: ReactNode
}

const depthStyles = [
  // Depth 0 (H1): outer card — prominent
  {
    elevation: 3,
    border: 1,
    borderColor: 'divider',
    borderRadius: 2,
    headerVariant: 'h6' as const,
    headerWeight: 700,
    headerBg: 'action.hover',
  },
  // Depth 1 (H2): inner card — subtle
  {
    elevation: 1,
    border: 1,
    borderColor: 'divider',
    borderRadius: 1.5,
    headerVariant: 'subtitle1' as const,
    headerWeight: 600,
    headerBg: 'transparent',
  },
  // Depth 2 (H3): sub-section — minimal
  {
    elevation: 0,
    border: 0,
    borderColor: 'transparent',
    borderRadius: 1,
    headerVariant: 'subtitle2' as const,
    headerWeight: 600,
    headerBg: 'transparent',
  },
]

function PanelCard({ depth, title, children }: PanelCardProps) {
  const style = depthStyles[Math.min(depth, depthStyles.length - 1)]

  return (
    <Paper
      elevation={style.elevation}
      sx={{
        border: style.border,
        borderColor: style.borderColor,
        borderRadius: style.borderRadius,
        overflow: 'hidden',
        ...(depth === 2 ? { borderLeft: 3, borderLeftColor: 'primary.main' } : {}),
      }}
    >
      {title ? (
        <Box
          sx={{
            px: 2,
            py: 1,
            bgcolor: style.headerBg,
            ...(depth < 2 ? { borderBottom: 1, borderColor: 'divider' } : {}),
          }}
        >
          <Typography variant={style.headerVariant} sx={{ fontWeight: style.headerWeight }}>
            {title}
          </Typography>
        </Box>
      ) : null}
      <Box sx={{ p: 1.5, display: 'flex', flexDirection: 'column', gap: 1 }}>{children}</Box>
    </Paper>
  )
}

export { PanelCard }
export type { PanelCardProps }
