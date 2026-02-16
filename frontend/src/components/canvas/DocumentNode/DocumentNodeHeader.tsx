import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { ContextNodeIcon as DocumentNodeIcon } from '../Icons/ContextNodeIcon'
import { ProtocolBadge } from '../ProtocolBadge'
import { DOCUMENT_NODE } from './constants'

type DocumentNodeHeaderProps = {
  name: string
  parentStepName: string
  accentColor?: string
}

function DocumentNodeHeader({ name, parentStepName, accentColor = DOCUMENT_NODE.ACCENT_COLOR }: DocumentNodeHeaderProps) {
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
          {parentStepName}
        </Typography>
      </Box>

      <Box sx={{ mr: 0.5 }}>
        <ProtocolBadge color={accentColor} label="Document" />
      </Box>
    </Box>
  )
}

export { DocumentNodeHeader }
export type { DocumentNodeHeaderProps }
