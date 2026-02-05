import { Box, Typography } from '@mui/material'
import type { FeedItem, FeedItemType } from '@/types'

type FeedStreamProps = {
  items: FeedItem[]
  maxVisible: number
}

const TYPE_ICON: Record<FeedItemType, string> = {
  agent_report: '>',
  task_started: '+',
  task_completed: '*',
  error: '!',
  user_message: '$',
  system_notice: '#',
  milestone: '@',
}

const TYPE_COLOR: Record<FeedItemType, string> = {
  agent_report: 'text.secondary',
  task_started: 'info.main',
  task_completed: 'success.main',
  error: 'error.main',
  user_message: 'text.secondary',
  system_notice: 'text.secondary',
  milestone: 'success.main',
}

const formatTime = (iso: string): string => {
  const d = new Date(iso)
  const h = String(d.getHours()).padStart(2, '0')
  const m = String(d.getMinutes()).padStart(2, '0')
  const s = String(d.getSeconds()).padStart(2, '0')
  return `${h}:${m}:${s}`
}

function FeedStream({ items, maxVisible }: FeedStreamProps) {
  const visible = items.slice(-maxVisible)

  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'flex-end',
        gap: 0,
        height: '12em',
        overflow: 'hidden',
        maskImage: 'linear-gradient(to bottom, transparent 0%, black 30%)',
        WebkitMaskImage: 'linear-gradient(to bottom, transparent 0%, black 30%)',
        fontSize: '0.75rem',
        lineHeight: 1.4,
      }}
    >
      {visible.map((item) => (
        <Box
          key={item.id}
          sx={{
            display: 'flex',
            gap: 1,
            py: '1px',
          }}
        >
          <Typography
            component="span"
            sx={{
              fontSize: 'inherit',
              lineHeight: 'inherit',
              color: 'text.disabled',
              flexShrink: 0,
              fontVariantNumeric: 'tabular-nums',
            }}
          >
            {formatTime(item.timestamp)}
          </Typography>
          <Typography
            component="span"
            sx={{
              fontSize: 'inherit',
              lineHeight: 'inherit',
              flexShrink: 0,
              width: '1ch',
              color: TYPE_COLOR[item.item_type],
            }}
          >
            {TYPE_ICON[item.item_type]}
          </Typography>
          <Typography
            component="span"
            sx={{
              fontSize: 'inherit',
              lineHeight: 'inherit',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              color: 'text.secondary',
            }}
          >
            {item.content}
          </Typography>
        </Box>
      ))}
    </Box>
  )
}

export { FeedStream }
export type { FeedStreamProps }
