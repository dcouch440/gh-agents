import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { NotesIcon } from '../Icons/NotesIcon'
import { ProtocolBadge } from '../ProtocolBadge'
import { NOTES_NODE } from './constants'

type NotesNodeHeaderProps = {
  name: string
  stepName: string
  accentColor?: string
}

function NotesNodeHeader({ name, stepName, accentColor = NOTES_NODE.ACCENT_COLOR }: NotesNodeHeaderProps) {
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
        <NotesIcon color={accentColor} size={18} />
      </Box>

      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Typography
          sx={{
            fontSize: 13,
            fontWeight: 600,
            color: 'text.primary',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {name}
        </Typography>
        <Typography
          sx={{
            fontSize: 10,
            color: 'text.disabled',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            lineHeight: 1.2,
          }}
        >
          {stepName}
        </Typography>
      </Box>

      <Box sx={{ mr: 0.5 }}>
        <ProtocolBadge color={accentColor} label="Notes" />
      </Box>
    </Box>
  )
}

export { NotesNodeHeader }
export type { NotesNodeHeaderProps }
