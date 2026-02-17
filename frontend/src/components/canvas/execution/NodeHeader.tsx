import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

import type { NodeHeaderProps } from './types'
import { SIZE_CONFIG } from './types'

function NodeHeader({
  icon,
  title,
  subtitle,
  accentColor,
  size = 'standard',
  badge,
  actions,
}: NodeHeaderProps) {
  const config = SIZE_CONFIG[size]

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: config.gap,
        px: 1.5,
        width: '100%',
        overflow: 'hidden',
      }}
    >
      <Box
        sx={{
          width: config.iconBox,
          height: config.iconBox,
          borderRadius: '6px',
          backgroundColor: `${accentColor}33`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
          fontSize: config.iconFont,
          color: accentColor,
        }}
      >
        {icon}
      </Box>

      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Typography
          sx={{
            fontSize: config.titleFont,
            fontWeight: 600,
            color: 'text.primary',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {title}
        </Typography>
        {subtitle !== null && (
          <Typography
            sx={{
              fontSize: 10,
              color: 'text.disabled',
              lineHeight: 1.2,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {subtitle}
          </Typography>
        )}
      </Box>

      {badge !== undefined && (
        <Box sx={{ flexShrink: 0 }}>
          {badge}
        </Box>
      )}

      {actions !== undefined && (
        <Box sx={{ flexShrink: 0 }}>
          {actions}
        </Box>
      )}
    </Box>
  )
}

export { NodeHeader }
export type { NodeHeaderProps }
