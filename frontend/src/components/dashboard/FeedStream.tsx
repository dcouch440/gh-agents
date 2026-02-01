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
    <div className="feed-stream">
      {visible.map((item) => (
        <div key={item.id} className="feed-stream__line">
          <span className="feed-stream__time">{formatTime(item.timestamp)}</span>
          <span className={`feed-stream__type feed-stream__type--${item.item_type}`}>
            {TYPE_ICON[item.item_type]}
          </span>
          <span className="feed-stream__content">{item.content}</span>
        </div>
      ))}
    </div>
  )
}

export { FeedStream }
export type { FeedStreamProps }
