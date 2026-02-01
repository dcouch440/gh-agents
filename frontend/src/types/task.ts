type TaskStatus = 'pending' | 'in_progress' | 'review' | 'completed' | 'failed'

type TaskPriority = 'low' | 'normal' | 'high' | 'urgent'

type Task = {
  id: string
  slice_id: string | null
  title: string
  description: string
  assigned_tier: string
  assigned_agent: string | null
  status: TaskStatus
  priority: TaskPriority
  context_files: string[]
  metadata: Record<string, string> | null
  depends_on: string[]
  retry_count: number
  max_retries: number
  last_error: string | null
  created_at: string
  updated_at: string
}

export type { Task, TaskStatus, TaskPriority }
