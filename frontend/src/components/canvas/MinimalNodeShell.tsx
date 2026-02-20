import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import { LOD } from './constants'

type MinimalNodeShellProps = {
  label: string
  accentColor: string
  borderColor: string
  boxShadow: string
}

function MinimalNodeShell({ label, borderColor, boxShadow }: MinimalNodeShellProps) {
  const theme = useTheme()

  return (
    <Box
      sx={{
        width: '100%',
        height: '100%',
        borderRadius: '8px',
        backgroundColor: theme.palette.custom.screenBg,
        border: 1,
        borderColor,
        boxShadow,
        overflow: 'hidden',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
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
