import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import AutoAwesome from '@mui/icons-material/AutoAwesome'
import Check from '@mui/icons-material/Check'
import DescriptionOutlined from '@mui/icons-material/DescriptionOutlined'
import type { ToolStatus } from '@/types'
import { getToolLabel } from './toolLabels'

type ToolIndicatorProps =
  | { variant: 'tool'; toolName: string; status: ToolStatus }
  | { variant: 'doc_update'; title: string }

function ToolIndicator(props: ToolIndicatorProps) {
  if (props.variant === 'doc_update') {
    return (
      <Box
        sx={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: 0.75,
          py: 0.5,
          px: 1,
          my: 0.5,
          borderLeft: 2,
          borderColor: 'info.main',
          borderRadius: '0 4px 4px 0',
          bgcolor: 'action.hover',
        }}
      >
        <DescriptionOutlined sx={{ fontSize: '0.875rem', color: 'info.main' }} />
        <Typography
          variant="body2"
          sx={{ fontFamily: 'monospace', fontSize: '0.75rem', color: 'text.secondary' }}
        >
          Updated &ldquo;{props.title}&rdquo;
        </Typography>
      </Box>
    )
  }

  const { toolName, status } = props
  const label = getToolLabel(toolName, status)
  const isRunning = status === 'running'

  return (
    <Box
      sx={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 0.75,
        py: 0.5,
        px: 1,
        my: 0.5,
        borderLeft: 2,
        borderColor: isRunning ? 'primary.main' : 'success.main',
        borderRadius: '0 4px 4px 0',
        bgcolor: 'action.hover',
        ...(isRunning && {
          animation: 'toolPulse 2s ease-in-out infinite',
          '@keyframes toolPulse': {
            '0%, 100%': { opacity: 1 },
            '50%': { opacity: 0.6 },
          },
        }),
      }}
    >
      {isRunning ? (
        <AutoAwesome sx={{ fontSize: '0.875rem', color: 'primary.main' }} />
      ) : (
        <Check sx={{ fontSize: '0.875rem', color: 'success.main' }} />
      )}
      <Typography
        variant="body2"
        sx={{ fontFamily: 'monospace', fontSize: '0.75rem', color: 'text.secondary' }}
      >
        {label}
      </Typography>
    </Box>
  )
}

export { ToolIndicator }
export type { ToolIndicatorProps }
