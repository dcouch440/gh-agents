import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import { statusColor } from '@/utils/statusColor'

import type { ExecutionStatus } from './types'
import { STATUS_LABELS } from './types'

type ExecutionStatusBadgeProps = {
  status: ExecutionStatus
}

function ExecutionStatusBadge({ status }: ExecutionStatusBadgeProps) {
  const theme = useTheme()
  const color = statusColor(status, theme.palette.statusPalette)
  const label = STATUS_LABELS[status]

  if (color === null || label === null) return null

  const animated = status === 'running'

  return (
    <Box
      sx={{
        flexShrink: 0,
        display: 'inline-flex',
        alignItems: 'center',
        gap: 0.75,
        px: 1.25,
        py: 0.5,
        borderRadius: '100px',
        background: `${color}12`,
        border: 1,
        borderColor: `${color}25`,
      }}
    >
      <Box
        sx={{
          width: 6,
          height: 6,
          borderRadius: '50%',
          backgroundColor: color,
          flexShrink: 0,
          position: 'relative',
          ...(animated && {
            '@keyframes protocolBadgePing': {
              '0%': { transform: 'scale(1)', opacity: 0.75 },
              '75%, 100%': { transform: 'scale(2)', opacity: 0 },
            },
            '&::after': {
              content: '""',
              position: 'absolute',
              inset: 0,
              borderRadius: '50%',
              backgroundColor: color,
              animation: 'protocolBadgePing 2s cubic-bezier(0, 0, 0.2, 1) infinite',
            },
          }),
        }}
      />
      <Typography
        sx={{
          fontSize: 9,
          fontWeight: 700,
          letterSpacing: '0.1em',
          textTransform: 'uppercase',
          color,
          lineHeight: 1,
          whiteSpace: 'nowrap',
        }}
      >
        {label}
      </Typography>
    </Box>
  )
}

export { ExecutionStatusBadge }
export type { ExecutionStatusBadgeProps }
