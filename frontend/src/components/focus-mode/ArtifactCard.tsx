import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import ButtonBase from '@mui/material/ButtonBase'
import { FOCUS_MODE, ANIMATION } from '@/constants'

type ArtifactCardProps = {
  name: string
  subtitle: string | null
  accentColor: string
  highlighted: boolean
  onClick: () => void
}

function ArtifactCard({ name, subtitle, accentColor, highlighted, onClick }: ArtifactCardProps) {
  return (
    <ButtonBase
      onClick={onClick}
      sx={{
        width: FOCUS_MODE.CARD_WIDTH,
        minWidth: FOCUS_MODE.CARD_WIDTH,
        height: FOCUS_MODE.CARD_HEIGHT,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'stretch',
        borderRadius: '6px',
        overflow: 'hidden',
        border: 1,
        borderColor: highlighted ? accentColor : 'divider',
        backgroundColor: highlighted ? 'background.paper' : 'background.default',
        opacity: highlighted ? 1 : 0.4,
        filter: highlighted ? 'none' : 'grayscale(0.3)',
        transform: highlighted ? 'scale(1.05)' : 'scale(1)',
        transition: `opacity ${ANIMATION.FAST}ms ease, transform ${ANIMATION.FAST}ms ease, border-color ${ANIMATION.FAST}ms ease, filter ${ANIMATION.FAST}ms ease`,
        '&:hover': {
          opacity: highlighted ? 1 : 0.7,
          filter: 'none',
        },
      }}
    >
      {/* Accent top bar */}
      <Box
        sx={{
          height: 4,
          flexShrink: 0,
          backgroundColor: accentColor,
        }}
      />

      {/* Content */}
      <Box
        sx={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'center',
          alignItems: 'center',
          px: 0.5,
          py: 0.5,
          gap: 0.25,
          minHeight: 0,
        }}
      >
        <Typography
          sx={{
            fontSize: 11,
            fontWeight: 600,
            lineHeight: 1.2,
            color: 'text.primary',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            width: '100%',
            textAlign: 'center',
          }}
        >
          {name}
        </Typography>
        {subtitle !== null && (
          <Typography
            sx={{
              fontSize: 9,
              lineHeight: 1.2,
              color: 'text.secondary',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              width: '100%',
              textAlign: 'center',
            }}
          >
            {subtitle}
          </Typography>
        )}
      </Box>
    </ButtonBase>
  )
}

export { ArtifactCard }
export type { ArtifactCardProps }
