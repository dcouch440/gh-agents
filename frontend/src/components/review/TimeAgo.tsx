import { Typography } from '@mui/material'

type TimeAgoProps = {
  timestamp: string
}

const formatTimeAgo = (timestamp: string): string => {
  const seconds = Math.floor((Date.now() - Date.parse(timestamp)) / 1000)

  if (seconds < 60) return 'just now'

  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}m ago`

  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}h ago`

  const days = Math.floor(hours / 24)
  if (days < 30) return `${days}d ago`

  const months = Math.floor(days / 30)
  return `${months}mo ago`
}

function TimeAgo({ timestamp }: TimeAgoProps) {
  return (
    <Typography variant="caption" color="text.secondary" component="span">
      {formatTimeAgo(timestamp)}
    </Typography>
  )
}

export { TimeAgo }
export type { TimeAgoProps }
