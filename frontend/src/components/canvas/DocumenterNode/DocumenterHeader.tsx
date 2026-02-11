import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { DocumenterIcon } from '@/components/canvas/Icons/DocumentIcon'
import { PROTOCOL_TYPE_COLORS } from '@/components/canvas/constants'
import { ProtocolBadge } from '@/components/canvas/ProtocolBadge'

type DocumenterHeaderProps = {
  name: string
  documentNames: string[]
}

const ACCENT = PROTOCOL_TYPE_COLORS.documenter

function DocumenterHeader({ name, documentNames }: DocumenterHeaderProps) {
  const docSummary = documentNames.length > 0 ? documentNames.join(' \u00b7 ') : null

  return (
    <Box
      sx={{
        width: '100%',
        height: '100%',
        display: 'flex',
        alignItems: 'center',
        gap: 1.5,
        px: 1.5,
        overflow: 'hidden',
      }}
    >
      {/* Icon */}
      <Box
        sx={{
          flexShrink: 0,
          width: 36,
          height: 36,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <DocumenterIcon />
      </Box>

      {/* Title + subtitle */}
      <Box
        sx={{
          flex: 1,
          minWidth: 0,
          display: 'flex',
          flexDirection: 'column',
          gap: 0.25,
        }}
      >
        <Typography
          sx={{
            fontSize: 14,
            fontWeight: 600,
            color: 'text.primary',
            lineHeight: 1.2,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {name}
        </Typography>
        <Typography
          sx={{
            fontSize: 11,
            color: docSummary !== null ? 'text.secondary' : 'text.disabled',
            lineHeight: 1.2,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {docSummary ?? 'No documents'}
        </Typography>
      </Box>

      <ProtocolBadge color={ACCENT} label="Protocol" animated />
    </Box>
  )
}

export { DocumenterHeader }
export type { DocumenterHeaderProps }
