import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'

type InputsTabProps = {
  upstreamStepNames: string[]
}

function InputsTab({ upstreamStepNames }: InputsTabProps) {
  const theme = useTheme()

  if (upstreamStepNames.length === 0) {
    return (
      <Box sx={{ p: 1.5, height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Typography sx={{ fontSize: 12, color: 'text.disabled' }}>No upstream connections</Typography>
      </Box>
    )
  }

  return (
    <Box sx={{ p: 1.5, display: 'flex', flexDirection: 'column', gap: 1 }}>
      <Typography
        sx={{
          fontSize: 8,
          fontWeight: 600,
          textTransform: 'uppercase',
          color: 'text.disabled',
          letterSpacing: '0.06em',
          lineHeight: 1,
        }}
      >
        Upstream Inputs
      </Typography>
      <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
        {upstreamStepNames.map((name, idx) => (
          <Box
            key={idx}
            sx={{
              display: 'inline-flex',
              alignItems: 'center',
              px: 0.75,
              py: 0.25,
              borderRadius: '4px',
              backgroundColor: theme.palette.custom.hoverOverlay,
              border: 1,
              borderColor: 'divider',
              fontSize: 10,
              color: 'text.secondary',
              lineHeight: 1.3,
              maxWidth: '100%',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {name}
          </Box>
        ))}
      </Box>
    </Box>
  )
}

export { InputsTab }
export type { InputsTabProps }
