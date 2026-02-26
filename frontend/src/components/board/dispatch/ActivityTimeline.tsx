import { useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import ExpandMoreIcon from '@mui/icons-material/ExpandMore'
import ExpandLessIcon from '@mui/icons-material/ExpandLess'
import { activityMessage } from '@/stores/activity/activityMessages'
import type { ActivityEntry } from '@/stores/activity'
import { relativeTime, isErrorEvent } from './utils'

type ActivityTimelineProps = {
  readonly activities: readonly ActivityEntry[]
}

/**
 * Compact, collapsible activity timeline.
 * Collapsed: last 5 events. Expanded: all events in a scrollable container.
 */
function ActivityTimeline({ activities }: ActivityTimelineProps) {
  const [expanded, setExpanded] = useState(false)

  if (activities.length === 0) return null

  const visible = expanded ? activities : activities.slice(-5)

  return (
    <Box sx={{ px: 1.5, py: 0.75 }}>
      <Box
        onClick={() => setExpanded((v) => !v)}
        sx={{ display: 'flex', alignItems: 'center', cursor: 'pointer', gap: 0.5, '&:hover': { opacity: 0.8 } }}
      >
        <IconButton size="small" sx={{ p: 0 }}>
          {expanded
            ? <ExpandLessIcon sx={{ fontSize: 14, color: 'text.secondary' }} />
            : <ExpandMoreIcon sx={{ fontSize: 14, color: 'text.secondary' }} />}
        </IconButton>
        <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary' }}>
          Activity
        </Typography>
        <Typography variant="caption" sx={{ color: 'text.disabled', fontFamily: 'monospace', fontSize: 11, ml: 'auto' }}>
          {activities.length} event(s)
        </Typography>
      </Box>

      <Box
        sx={{
          pl: 3,
          mt: 0.5,
          display: 'flex',
          flexDirection: 'column',
          gap: 0.25,
          ...(expanded && { maxHeight: 200, overflowY: 'auto' }),
        }}
      >
        {visible.map((entry) => (
          <ActivityTimelineRow key={entry.id} entry={entry} />
        ))}
      </Box>
    </Box>
  )
}

// ── Row ──────────────────────────────────────────────────────────────────────

function ActivityTimelineRow({ entry }: { readonly entry: ActivityEntry }) {
  const message = activityMessage(entry.event)
  const isError = isErrorEvent(entry.event.type)

  return (
    <Typography
      variant="caption"
      sx={{
        fontFamily: 'monospace',
        fontSize: 11,
        lineHeight: 1.5,
        color: isError ? 'error.main' : 'text.secondary',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap',
      }}
    >
      <Box component="span" sx={{ color: 'text.disabled', mr: 0.5 }}>
        {relativeTime(entry.ts)}
      </Box>
      {message}
    </Typography>
  )
}

export { ActivityTimeline }
export type { ActivityTimelineProps }
