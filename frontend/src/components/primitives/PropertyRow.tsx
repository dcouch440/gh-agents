import type { ReactNode } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

type PropertyRowProps = {
  label: string
  value?: string | null
  mono?: boolean
  last?: boolean
  children?: ReactNode | null
}

function PropertyRow({ label, value = null, mono = false, last = false, children = null }: PropertyRowProps) {
  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        px: '16px',
        pt: '6px',
        pb: last ? '12px' : '6px',
        transition: 'background-color 100ms ease',
        '&:hover': { backgroundColor: (theme) => theme.palette.custom.hoverOverlay },
      }}
    >
      <Typography
        sx={{
          fontSize: 11,
          color: 'text.secondary',
          flexShrink: 0,
          mr: 2,
        }}
      >
        {label}
      </Typography>
      {children !== null ? (
        <Box sx={{ display: 'flex', alignItems: 'center', minWidth: 0 }}>{children}</Box>
      ) : (
        <Typography
          sx={{
            fontSize: 11,
            fontWeight: 500,
            color: 'text.primary',
            fontFamily: mono ? 'monospace' : 'inherit',
            textAlign: 'right',
            minWidth: 0,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {value}
        </Typography>
      )}
    </Box>
  )
}

export { PropertyRow }
export type { PropertyRowProps }
