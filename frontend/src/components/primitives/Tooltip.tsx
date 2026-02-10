import { type ReactElement } from 'react'
import MuiTooltip from '@mui/material/Tooltip'
import Box from '@mui/material/Box'

type TooltipProps = {
  title: string
  shortcut?: string
  placement?: 'top' | 'bottom' | 'left' | 'right'
  children: ReactElement
}

function Tooltip({ title, shortcut, placement = 'bottom', children }: TooltipProps) {
  const label = shortcut ? (
    <Box sx={{ display: 'inline-flex', alignItems: 'center', gap: 1 }}>
      <span>{title}</span>
      <Box
        component="kbd"
        sx={{
          fontSize: '0.625rem',
          fontFamily: 'inherit',
          lineHeight: 1,
          px: 0.5,
          py: '1px',
          borderRadius: '4px',
          backgroundColor: 'rgba(255, 255, 255, 0.08)',
          border: '1px solid rgba(255, 255, 255, 0.1)',
          color: 'text.secondary',
        }}
      >
        {shortcut}
      </Box>
    </Box>
  ) : (
    title
  )

  return (
    <MuiTooltip title={label} placement={placement}>
      {children}
    </MuiTooltip>
  )
}

export { Tooltip }
export type { TooltipProps }
