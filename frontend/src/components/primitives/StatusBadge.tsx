import { Chip } from '@mui/material'

type BadgeVariant = 'success' | 'warning' | 'error' | 'info' | 'neutral'

type StatusBadgeProps = {
  label: string
  variant: BadgeVariant
}

const VARIANT_COLOR_MAP = {
  success: 'success',
  error: 'error',
  warning: 'warning',
  info: 'info',
  neutral: 'default',
} as const

function StatusBadge({ label, variant }: StatusBadgeProps) {
  return (
    <Chip
      label={label}
      color={VARIANT_COLOR_MAP[variant]}
      size="small"
      sx={{ fontWeight: 500 }}
    />
  )
}

export { StatusBadge }
export type { BadgeVariant, StatusBadgeProps }
