import { Fragment } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import { FOCUS_MODE } from '@/constants'
import { ArtifactCard } from './ArtifactCard'

// ── Types ────────────────────────────────────────────────────────────────────

type CardEntry = {
  id: string
  name: string
  subtitle: string | null
  accentOverride: string | null
}

type StepSection = {
  stepId: string
  stepName: string
  accentColor: string
  cards: readonly CardEntry[]
}

type ArtifactBarProps = {
  sections: readonly StepSection[]
  currentStepId: string | null
  onStepClick: (stepId: string) => void
}

// ── Component ────────────────────────────────────────────────────────────────

function ArtifactBar({ sections, currentStepId, onStepClick }: ArtifactBarProps) {
  const theme = useTheme()

  if (sections.length === 0) return null

  return (
    <Box
      sx={{
        height: FOCUS_MODE.ARTIFACT_BAR_HEIGHT,
        minHeight: FOCUS_MODE.ARTIFACT_BAR_HEIGHT,
        display: 'flex',
        alignItems: 'center',
        gap: 2,
        pl: 1.5,
        pr: 6,
        py: 1,
        overflowX: 'auto',
        overflowY: 'hidden',
        borderBottom: 1,
        borderColor: 'divider',
        backgroundColor: theme.palette.custom.chromeBg,
        '&::-webkit-scrollbar': {
          height: 4,
        },
        '&::-webkit-scrollbar-thumb': {
          backgroundColor: theme.palette.divider,
          borderRadius: 2,
        },
      }}
    >
      {sections.map((section, si) => (
        <Fragment key={section.stepId}>
          {si > 0 && (
            <Box sx={{ width: '1px', height: 48, backgroundColor: 'divider', flexShrink: 0 }} />
          )}
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flexShrink: 0 }}>
            <Typography
              sx={{
                fontSize: 9,
                fontWeight: 700,
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'text.disabled',
                writingMode: 'vertical-rl',
                transform: 'rotate(180deg)',
                whiteSpace: 'nowrap',
              }}
            >
              {section.stepName}
            </Typography>
            {section.cards.map((card) => (
              <ArtifactCard
                key={card.id}
                name={card.name}
                subtitle={card.subtitle}
                accentColor={card.accentOverride ?? section.accentColor}
                highlighted={section.stepId === currentStepId}
                onClick={() => {
                  onStepClick(section.stepId)
                }}
              />
            ))}
          </Box>
        </Fragment>
      ))}
    </Box>
  )
}

export { ArtifactBar }
export type { ArtifactBarProps, StepSection, CardEntry }
