import Typography from '@mui/material/Typography'
import Box from '@mui/material/Box'
import { activityMessage } from '@/stores/activity/activityMessages'
import { relativeTime, isErrorEvent } from './utils'
import type { ActivityEntry as ActivityEntryType } from '@/stores/activity'

type ActivityEntryProps = {
  readonly entry: ActivityEntryType
  readonly referenceMs: number
}

function ActivityEntry({ entry, referenceMs }: ActivityEntryProps) {
  const isError = isErrorEvent(entry.event.type)

  return (
    <Box sx={{ display: 'flex', gap: 1, py: 0.25, fontFamily: 'monospace' }}>
      <Typography
        variant="caption"
        sx={{
          color: 'text.disabled',
          flexShrink: 0,
          fontFamily: 'inherit',
          fontSize: 11,
          lineHeight: 1.6,
          minWidth: 56,
          textAlign: 'right',
        }}
      >
        {relativeTime(entry.receivedAt, referenceMs)}
      </Typography>
      <Typography
        variant="caption"
        sx={{
          color: isError ? 'error.main' : 'text.secondary',
          fontFamily: 'inherit',
          fontSize: 11,
          lineHeight: 1.6,
          wordBreak: 'break-word',
        }}
      >
        {activityMessage(entry.event)}
      </Typography>
    </Box>
  )
}

export { ActivityEntry }
export type { ActivityEntryProps }
