import Box from '@mui/material/Box'
import IconButton from '@mui/material/IconButton'
import Typography from '@mui/material/Typography'
import ChevronLeftOutlined from '@mui/icons-material/ChevronLeftOutlined'
import ChevronRightOutlined from '@mui/icons-material/ChevronRightOutlined'
import { useTheme } from '@mui/material/styles'
import { FOCUS_MODE, ANIMATION } from '@/constants'

type FocusNavBarProps = {
  stepCount: number
  currentIndex: number
  currentStepName: string
  accentColors: string[]
  onPrev: () => void
  onNext: () => void
  onDotClick: (index: number) => void
}

function FocusNavBar({
  stepCount,
  currentIndex,
  currentStepName,
  accentColors,
  onPrev,
  onNext,
  onDotClick,
}: FocusNavBarProps) {
  const theme = useTheme()

  return (
    <Box
      sx={{
        height: FOCUS_MODE.NAV_BAR_HEIGHT,
        minHeight: FOCUS_MODE.NAV_BAR_HEIGHT,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 1.5,
        px: 1,
        borderTop: 1,
        borderColor: 'divider',
        backgroundColor: theme.palette.custom.chromeBg,
        backdropFilter: 'blur(8px)',
        flexShrink: 0,
      }}
    >
      {/* Prev button */}
      <IconButton
        onClick={onPrev}
        disabled={currentIndex <= 0}
        size="small"
        sx={{
          width: 32,
          height: 32,
          color: 'text.secondary',
          '&:hover': { color: 'text.primary' },
        }}
      >
        <ChevronLeftOutlined sx={{ fontSize: 20 }} />
      </IconButton>

      {/* Center: step name + dots */}
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 0.5,
          minWidth: 0,
        }}
      >
        <Typography
          sx={{
            fontSize: 11,
            color: 'text.secondary',
            fontWeight: 500,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            maxWidth: 200,
          }}
        >
          {currentStepName}
        </Typography>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.75 }}>
          {Array.from({ length: stepCount }, (_, i) => {
            const isActive = i === currentIndex
            const dotColor = accentColors[i] ?? theme.palette.text.disabled
            return (
              <Box
                key={i}
                onClick={() => {
                  onDotClick(i)
                }}
                sx={{
                  width: isActive ? 10 : 6,
                  height: isActive ? 10 : 6,
                  borderRadius: '50%',
                  backgroundColor: isActive ? dotColor : theme.palette.text.disabled,
                  cursor: 'pointer',
                  transition: `width ${ANIMATION.FAST}ms ease, height ${ANIMATION.FAST}ms ease, background-color ${ANIMATION.FAST}ms ease`,
                  '&:hover': {
                    backgroundColor: dotColor,
                  },
                }}
              />
            )
          })}
        </Box>
      </Box>

      {/* Next button */}
      <IconButton
        onClick={onNext}
        disabled={currentIndex >= stepCount - 1}
        size="small"
        sx={{
          width: 32,
          height: 32,
          color: 'text.secondary',
          '&:hover': { color: 'text.primary' },
        }}
      >
        <ChevronRightOutlined sx={{ fontSize: 20 }} />
      </IconButton>
    </Box>
  )
}

export { FocusNavBar }
export type { FocusNavBarProps }
