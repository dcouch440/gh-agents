import Box from '@mui/material/Box'
import IconButton from '@mui/material/IconButton'
import Tooltip from '@mui/material/Tooltip'
import Typography from '@mui/material/Typography'
import OpenInFullOutlined from '@mui/icons-material/OpenInFullOutlined'
import { ProtocolBadge } from '@/components/canvas/ProtocolBadge'
import { Archetype, ARCHETYPE_CONFIGS } from './archetypes'
import type { Archetype as ArchetypeType } from './archetypes'

type DynamicNodeHeaderProps = {
  name: string
  archetype: ArchetypeType
  subtitle: string | null
  issueCount?: number
  issueDescriptions?: string[]
  onExpand?: () => void
}

const ISSUE_COLOR = '#f85149'

function DynamicNodeHeader({ name, archetype, subtitle, issueCount, issueDescriptions, onExpand }: DynamicNodeHeaderProps) {
  const config = ARCHETYPE_CONFIGS[archetype]
  const IconComponent = config.icon
  const hasIssues = issueCount !== undefined && issueCount > 0

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
        <IconComponent sx={{ fontSize: 20, color: config.color }} />
      </Box>

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
            color: subtitle !== null ? 'text.secondary' : 'text.disabled',
            lineHeight: 1.2,
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
          placement="top"
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

      {onExpand !== undefined && (
        <IconButton
          className="nodrag"
          onClick={onExpand}
          size="small"
          sx={{
            flexShrink: 0,
            width: 28,
            height: 28,
            color: 'text.secondary',
            '&:hover': { color: 'text.primary' },
          }}
        >
          <OpenInFullOutlined sx={{ fontSize: 16 }} />
        </IconButton>
      )}
    </Box>
  )
}

export { DynamicNodeHeader }
export type { DynamicNodeHeaderProps }
