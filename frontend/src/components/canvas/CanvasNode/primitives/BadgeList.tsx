import type { SxProps, Theme } from '@mui/material/styles'
import Box from '@mui/material/Box'

type BadgeListProps = {
  items: readonly string[]
  badgeSx?: SxProps<Theme>
}

const defaultBadgeSx: SxProps<Theme> = {
  display: 'inline-flex',
  alignItems: 'center',
  gap: 0.5,
  px: 0.75,
  py: 0.25,
  borderRadius: '4px',
  backgroundColor: (theme) => theme.palette.custom.hoverOverlay,
  border: 1,
  borderColor: 'divider',
  fontSize: 10,
  color: 'text.secondary',
  lineHeight: 1.3,
  maxWidth: '100%',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
}

function BadgeList({ items, badgeSx }: BadgeListProps) {
  return (
    <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
      {items.map((name, idx) => (
        <Box key={idx} sx={badgeSx ?? defaultBadgeSx}>
          {name}
        </Box>
      ))}
    </Box>
  )
}

export { BadgeList }
export type { BadgeListProps }
