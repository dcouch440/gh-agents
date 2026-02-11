import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { ContextNodeIcon } from '../Icons/ContextNodeIcon'
import { ProtocolBadge } from '../ProtocolBadge'
import { CONTEXT_NODE } from './constants'

type ContextNodeHeaderProps = {
  name: string
  accentColor?: string
}

function ContextNodeHeader({ name, accentColor = CONTEXT_NODE.ACCENT_COLOR }: ContextNodeHeaderProps) {
  return (
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, px: 1.5, width: '100%' }}>
      <Box
        sx={{
          width: 28,
          height: 28,
          borderRadius: '6px',
          backgroundColor: `${accentColor}20`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
        }}
      >
        <ContextNodeIcon color={accentColor} size={18} />
      </Box>

      <Typography
        sx={{
          fontSize: 13,
          fontWeight: 600,
          color: 'text.primary',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
          flex: 1,
          minWidth: 0,
        }}
      >
        {name}
      </Typography>

      <Box sx={{ mr: 0.5 }}>
        <ProtocolBadge color={accentColor} label="Context" animated />
      </Box>
    </Box>
  )
}

export { ContextNodeHeader }
export type { ContextNodeHeaderProps }
