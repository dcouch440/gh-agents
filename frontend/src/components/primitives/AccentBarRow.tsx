import type { ReactNode } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { AccentBar } from './AccentBar'

type AccentBarRowProps = {
  barColor: string
  primary: string
  secondary?: string | null
  highlight?: boolean
  actions?: ReactNode | null
  onClick?: (() => void) | null
  children?: ReactNode | null
}

function AccentBarRow({
  barColor,
  primary,
  secondary = null,
  highlight = false,
  actions = null,
  onClick = null,
  children = null,
}: AccentBarRowProps) {
  return (
    <Box
      onClick={onClick ?? undefined}
      sx={{
        display: 'flex',
        alignItems: 'stretch',
        borderBottom: (theme) => `1px solid ${theme.palette.custom.separatorSubtle}`,
        backgroundColor: (theme) => highlight ? theme.palette.custom.activeTint : 'transparent',
        cursor: onClick !== null ? 'pointer' : 'default',
        transition: 'background-color 100ms ease',
        '&:hover': { backgroundColor: (theme) => theme.palette.custom.hoverOverlay },
      }}
    >
      <AccentBar color={barColor} />
      <Box
        sx={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          py: '10px',
          pr: '16px',
          pl: '12px',
          minWidth: 0,
        }}
      >
        {children ?? (
          <Box sx={{ minWidth: 0, flex: 1 }}>
            <Typography
              sx={{
                fontSize: 11,
                fontWeight: 500,
                color: 'text.primary',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              {primary}
            </Typography>
            {secondary !== null ? (
              <Typography
                sx={{
                  fontSize: 10,
                  color: 'text.secondary',
                  mt: '1px',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                {secondary}
              </Typography>
            ) : null}
          </Box>
        )}
        {actions !== null ? (
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, ml: 1, flexShrink: 0 }}>
            {actions}
          </Box>
        ) : null}
      </Box>
    </Box>
  )
}

export { AccentBarRow }
export type { AccentBarRowProps }
