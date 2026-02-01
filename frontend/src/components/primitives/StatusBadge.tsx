type BadgeVariant = 'success' | 'warning' | 'error' | 'info' | 'neutral'

type StatusBadgeProps = {
  label: string
  variant: BadgeVariant
}

function StatusBadge({ label, variant }: StatusBadgeProps) {
  return <span className={`badge badge--${variant}`}>{label}</span>
}

export { StatusBadge }
export type { BadgeVariant, StatusBadgeProps }
