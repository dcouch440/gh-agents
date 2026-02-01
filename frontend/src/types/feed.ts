type FeedItemType =
  | 'agent_report'
  | 'task_started'
  | 'task_completed'
  | 'error'
  | 'user_message'
  | 'system_notice'
  | 'milestone'

type VerbosityLevel = 'quiet' | 'normal' | 'verbose'

type FeedItem = {
  id: string
  agent_id: string
  content: string
  item_type: FeedItemType
  verbosity_level: VerbosityLevel
  timestamp: string
}

export type { FeedItem, FeedItemType, VerbosityLevel }
