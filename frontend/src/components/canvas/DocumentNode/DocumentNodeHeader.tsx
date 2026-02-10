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

      <Box
        sx={{
          flexShrink: 0,
          display: 'inline-flex',
          alignItems: 'center',
          gap: 0.75,
          px: 1.25,
          py: 0.5,
          borderRadius: '100px',
          background: `linear-gradient(135deg, ${accentColor}14, ${accentColor}22)`,
          border: 1,
          borderColor: `${accentColor}35`,
          boxShadow: `0 0 8px ${accentColor}18, inset 0 1px 0 ${accentColor}10`,
          mr: 0.5,
        }}
      >
        <Box
          sx={{
            width: 6,
            height: 6,
            borderRadius: '50%',
            backgroundColor: accentColor,
            boxShadow: `0 0 4px ${accentColor}80`,
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
              backgroundColor: accentColor,
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
            color: accentColor,
            lineHeight: 1,
            whiteSpace: 'nowrap',
          }}
        >
          Document
        </Typography>
      </Box>
    </Box>
  )
}

export { DocumentNodeHeader }
export type { DocumentNodeHeaderProps }
