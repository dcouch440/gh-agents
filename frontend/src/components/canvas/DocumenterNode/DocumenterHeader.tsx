import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { DocumenterIcon } from '@/components/canvas/Icons/DocumentIcon'
import { PROTOCOL_TYPE_COLORS } from '@/components/canvas/constants'

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

      {/* Protocol badge */}
      <Box
        sx={{
          flexShrink: 0,
          display: 'inline-flex',
          alignItems: 'center',
          gap: 0.75,
          px: 1.25,
          py: 0.5,
          borderRadius: '100px',
          background: `linear-gradient(135deg, ${ACCENT}14, ${ACCENT}22)`,
          border: 1,
          borderColor: `${ACCENT}35`,
          boxShadow: `0 0 8px ${ACCENT}18, inset 0 1px 0 ${ACCENT}10`,
        }}
      >
        <Box
          sx={{
            width: 6,
            height: 6,
            borderRadius: '50%',
            backgroundColor: ACCENT,
            boxShadow: `0 0 4px ${ACCENT}80`,
            flexShrink: 0,
            position: 'relative',
            '@keyframes ping': {
              '0%': { transform: 'scale(1)', opacity: 0.75 },
              '75%, 100%': { transform: 'scale(2)', opacity: 0 },
            },
            '&::after': {
              content: '""',
              position: 'absolute',
              inset: 0,
              borderRadius: '50%',
              backgroundColor: ACCENT,
              animation: 'ping 2s cubic-bezier(0, 0, 0.2, 1) infinite',
            },
          }}
        />
        <Typography
          sx={{
            fontSize: 9,
            fontWeight: 700,
            letterSpacing: '0.1em',
            textTransform: 'uppercase',
            color: ACCENT,
            lineHeight: 1,
            whiteSpace: 'nowrap',
          }}
        >
          Protocol
        </Typography>
      </Box>
    </Box>
  )
}

export { DocumenterHeader }
export type { DocumenterHeaderProps }
