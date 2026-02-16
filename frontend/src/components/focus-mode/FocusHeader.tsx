import Box from '@mui/material/Box'
import Tooltip from '@mui/material/Tooltip'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import { ProtocolBadge } from '@/components/canvas/ProtocolBadge'
import { Archetype, ARCHETYPE_CONFIGS } from '@/components/canvas/DynamicNode/archetypes'
import type { Archetype as ArchetypeType } from '@/components/canvas/DynamicNode/archetypes'
import { FOCUS_MODE } from '@/constants'

const ISSUE_COLOR = '#f85149'

type FocusHeaderProps = {
  name: string
  archetype: ArchetypeType
  subtitle: string | null
  issueCount?: number
  issueDescriptions?: string[]
}

function FocusHeader({ name, archetype, subtitle, issueCount, issueDescriptions }: FocusHeaderProps) {
  const theme = useTheme()
  const config = ARCHETYPE_CONFIGS[archetype]
  const IconComponent = config.icon
  const hasIssues = issueCount !== undefined && issueCount > 0

  return (
    <Box
      sx={{
        height: FOCUS_MODE.HEADER_HEIGHT,
        display: 'flex',
        alignItems: 'center',
        gap: 2,
        px: 3,
        borderBottom: 1,
        borderColor: 'divider',
        backgroundColor: theme.palette.custom.bgHeader,
        flexShrink: 0,
      }}
    >
      <Box
        sx={{
          flexShrink: 0,
          width: 40,
          height: 40,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <IconComponent sx={{ fontSize: 24, color: config.color }} />
      </Box>

      <Box
        sx={{
          flex: 1,
          minWidth: 0,
          display: 'flex',
          flexDirection: 'column',
          gap: 0.5,
        }}
      >
        <Typography
          sx={{
            fontSize: 18,
            fontWeight: 600,
            color: 'text.primary',
            lineHeight: 1.3,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {name}
        </Typography>
        <Typography
          sx={{
            fontSize: 13,
            color: subtitle !== null ? 'text.secondary' : 'text.disabled',
            lineHeight: 1.3,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {subtitle ?? (archetype === Archetype.BLANK ? 'Unconfigured' : `No ${config.archetypeTabLabel.toLowerCase()}`)}
        </Typography>
      </Box>

      {hasIssues ? (
        <Tooltip
          title={
            <Box sx={{ py: 0.5 }}>
              {(issueDescriptions ?? []).map((desc, i) => (
                <Typography key={i} sx={{ fontSize: 12, lineHeight: 1.4 }}>
                  {desc}
                </Typography>
              ))}
            </Box>
          }
          arrow
          placement="bottom"
        >
          <span>
            <ProtocolBadge
              color={ISSUE_COLOR}
              label={`${issueCount} Issue${issueCount > 1 ? 's' : ''}`}
              animated
            />
          </span>
        </Tooltip>
      ) : archetype !== Archetype.BLANK ? (
        <ProtocolBadge color={config.color} label={config.label} animated />
      ) : null}
    </Box>
  )
}

export { FocusHeader }
export type { FocusHeaderProps }
