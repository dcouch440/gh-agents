import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { LOD } from './constants'

type MinimalNodeShellProps = {
  label: string
  accentColor: string
  borderColor: string
  boxShadow: string
}

function MinimalNodeShell({ label, accentColor, borderColor, boxShadow }: MinimalNodeShellProps) {
  return (
    <Box
      sx={{
        width: '100%',
        height: '100%',
        borderRadius: '12px',
        backgroundColor: 'background.paper',
        border: 2,
        borderColor,
        boxShadow,
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'row',
      }}
    >
      <Box
        sx={{
          width: LOD.ACCENT_STRIPE_WIDTH,
          backgroundColor: accentColor,
          flexShrink: 0,
        }}
      />
      <Box
        sx={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          p: 2,
          overflow: 'hidden',
        }}
      >
        <Typography
          sx={{
            fontSize: LOD.MINIMAL_LABEL_FONT_SIZE,
            fontWeight: 700,
            color: 'text.primary',
            textAlign: 'center',
            overflow: 'hidden',
            display: '-webkit-box',
            WebkitLineClamp: 3,
            WebkitBoxOrient: 'vertical',
            wordBreak: 'break-word',
            lineHeight: 1.2,
            width: '100%',
          }}
        >
          {label}
        </Typography>
      </Box>
    </Box>
  )
}

export { MinimalNodeShell }
export type { MinimalNodeShellProps }
