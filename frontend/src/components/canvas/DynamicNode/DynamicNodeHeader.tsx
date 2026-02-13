import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { ProtocolBadge } from '@/components/canvas/ProtocolBadge'
import { Archetype, ARCHETYPE_CONFIGS } from './archetypes'
import type { Archetype as ArchetypeType } from './archetypes'

type DynamicNodeHeaderProps = {
  name: string
  archetype: ArchetypeType
  subtitle: string | null
}

function DynamicNodeHeader({ name, archetype, subtitle }: DynamicNodeHeaderProps) {
  const config = ARCHETYPE_CONFIGS[archetype]
  const IconComponent = config.icon

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

      {archetype !== Archetype.BLANK && (
        <ProtocolBadge color={config.color} label={config.label} animated />
      )}
    </Box>
  )
}

export { DynamicNodeHeader }
export type { DynamicNodeHeaderProps }
