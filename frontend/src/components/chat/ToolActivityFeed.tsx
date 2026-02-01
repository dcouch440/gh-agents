import { ToolActivityBox } from './ToolActivityBox'
import type { ToolStatus } from './ToolActivityBox'

type ToolEvent = {
  id: string
  toolName: string
  status: ToolStatus
  startedAt: number
  completedAt: number | null
  detail?: string
}

type ToolActivityFeedProps = {
  events: ToolEvent[]
  hint?: string | null
  now: number
}

const getDuration = (event: ToolEvent, now: number): number | null => {
  if (event.completedAt !== null) return event.completedAt - event.startedAt
  if (event.status === 'running') return now - event.startedAt
  return null
}

function ToolActivityFeed({ events, hint, now }: ToolActivityFeedProps) {
  return (
    <div className="tool-feed">
      {events.map((event) => (
        <ToolActivityBox
          key={event.id}
          toolName={event.toolName}
          status={event.status}
          durationMs={getDuration(event, now)}
          detail={event.detail}
        />
      ))}
      {hint ? <div className="tool-feed__hint">{hint}</div> : null}
    </div>
  )
}

export { ToolActivityFeed }
export type { ToolEvent, ToolActivityFeedProps }
