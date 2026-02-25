import { useRef, useEffect, useMemo } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useStore } from '@/stores/lib'
import { activityStore } from '@/stores/activity'
import { ActivityEntry } from './ActivityEntry'

/**
 * Scrollable list of activity events from the flight recorder.
 * Auto-scrolls to bottom unless the user has scrolled up manually.
 */
function ActivityFeed() {
  const entries = useStore(activityStore.store, activityStore.selectAll)
  const scrollRef = useRef<HTMLDivElement>(null)
  const isAutoScrolling = useRef(true)

  // Reference time = first entry's receivedAt, or now
  const referenceMs = useMemo(
    () => (entries.length > 0 ? entries[0]!.receivedAt : Date.now()),
    // Only recompute when the first entry appears
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [entries.length > 0],
  )

  // Auto-scroll to bottom when new entries arrive
  useEffect(() => {
    const el = scrollRef.current
    if (el && isAutoScrolling.current) {
      el.scrollTop = el.scrollHeight
    }
  }, [entries.length])

  const handleScroll = () => {
    const el = scrollRef.current
    if (!el) return
    // If user is within 40px of bottom, re-enable auto-scroll
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40
    isAutoScrolling.current = atBottom
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', minHeight: 0, flex: 1 }}>
      <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', mb: 0.5, px: 0.5 }}>
        Activity Feed
      </Typography>
      <Box
        ref={scrollRef}
        onScroll={handleScroll}
        sx={{
          flex: 1,
          minHeight: 0,
          overflowY: 'auto',
          px: 0.5,
        }}
      >
        {entries.length === 0 && (
          <Typography variant="caption" sx={{ color: 'text.disabled', fontFamily: 'monospace', fontSize: 11 }}>
            No events yet
          </Typography>
        )}
        {entries.map((entry) => (
          <ActivityEntry key={entry.id} entry={entry} referenceMs={referenceMs} />
        ))}
      </Box>
    </Box>
  )
}

export { ActivityFeed }
