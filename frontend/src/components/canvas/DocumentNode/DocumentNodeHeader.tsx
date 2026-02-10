import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { DocumentNodeIcon } from '../Icons/DocumentNodeIcon'
import { DOCUMENT_NODE } from './constants'

type DocumentNodeHeaderProps = {
  name: string
  accentColor?: string
}

function DocumentNodeHeader({ name, accentColor = DOCUMENT_NODE.ACCENT_COLOR }: DocumentNodeHeaderProps) {
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
        <DocumentNodeIcon color={accentColor} size={18} />
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

      <Typography
        sx={{
          fontSize: 9,
          fontWeight: 600,
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
          color: accentColor,
          opacity: 0.7,
          flexShrink: 0,
          pr: 0.5,
        }}
      >
        (Document)
      </Typography>
    </Box>
  )
}

export { DocumentNodeHeader }
export type { DocumentNodeHeaderProps }
