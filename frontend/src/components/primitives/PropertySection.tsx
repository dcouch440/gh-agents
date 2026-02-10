import type { ReactNode } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Collapse from '@mui/material/Collapse'
import KeyboardArrowDownRounded from '@mui/icons-material/KeyboardArrowDownRounded'
import { ANIMATION } from '@/constants'

type PropertySectionProps = {
  title: string | null
  open?: boolean
  onToggle?: (() => void) | null
  children: ReactNode
}

function PropertySection({ title, open = true, onToggle = null, children }: PropertySectionProps) {
  const isCollapsible = onToggle !== null

  return (
    <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
      {title !== null ? (
        <Box
          onClick={isCollapsible ? onToggle : undefined}
          sx={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            px: '16px',
            pt: '12px',
            pb: '8px',
            cursor: isCollapsible ? 'pointer' : 'default',
            userSelect: 'none',
          }}
        >
          <Typography
            sx={{
              fontSize: 11,
              fontWeight: 700,
              letterSpacing: '0.06em',
              textTransform: 'uppercase',
              color: 'text.secondary',
              lineHeight: 1,
            }}
          >
            {title}
          </Typography>
          {isCollapsible ? (
            <KeyboardArrowDownRounded
              sx={{
                fontSize: 16,
                color: 'text.disabled',
                transition: `transform ${ANIMATION.FAST}ms ease`,
                transform: open ? 'rotate(0deg)' : 'rotate(-90deg)',
              }}
            />
          ) : null}
        </Box>
      ) : null}
      {isCollapsible ? <Collapse in={open}>{children}</Collapse> : children}
    </Box>
  )
}

export { PropertySection }
export type { PropertySectionProps }
