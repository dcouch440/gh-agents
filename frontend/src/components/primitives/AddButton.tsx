import type { ReactNode } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import AddRounded from '@mui/icons-material/AddRounded'

type AddButtonProps = {
  label: string
  onClick: () => void
  icon?: ReactNode | null
}

function AddButton({ label, onClick, icon = null }: AddButtonProps) {
  return (
    <Box
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') onClick()
      }}
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 0.75,
        px: '16px',
        py: '10px',
        cursor: 'pointer',
        userSelect: 'none',
        transition: 'all 120ms ease',
        color: 'text.secondary',
        '&:hover': {
          color: 'primary.main',
          backgroundColor: (theme) => theme.palette.custom.activeTintStrong,
        },
      }}
    >
      {icon ?? <AddRounded sx={{ fontSize: 14 }} />}
      <Typography sx={{ fontSize: 11, fontWeight: 500 }}>{label}</Typography>
    </Box>
  )
}

export { AddButton }
export type { AddButtonProps }
