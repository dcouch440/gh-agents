import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

type ProtocolBadgeProps = {
  color: string
  label: string
  animated?: boolean
}

function ProtocolBadge({ color, label, animated = false }: ProtocolBadgeProps) {
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
        data-testid="protocol-badge-dot"
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

export { ProtocolBadge }
export type { ProtocolBadgeProps }
