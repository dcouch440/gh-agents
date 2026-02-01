import type { Task, TaskStatus, TaskPriority } from '@/types'

type TaskQueueStatusProps = {
  tasks: Task[]
}

const PRIORITY_MARKER: Record<TaskPriority, string> = {
  urgent: '!!!!',
  high: '!!!',
  normal: '!',
  low: '.',
}

const PRIORITY_ORDER: Record<TaskPriority, number> = {
  urgent: 0,
  high: 1,
  normal: 2,
  low: 3,
}

const STATUS_COUNTS: TaskStatus[] = ['pending', 'in_progress', 'review', 'completed', 'failed']

const STATUS_LABEL: Record<TaskStatus, string> = {
  pending: 'pending',
  in_progress: 'active',
  review: 'review',
  completed: 'done',
  failed: 'failed',
}

const countByStatus = (tasks: Task[]): Record<TaskStatus, number> => {
  const counts: Record<TaskStatus, number> = { pending: 0, in_progress: 0, review: 0, completed: 0, failed: 0 }
  for (const t of tasks) counts[t.status]++
  return counts
}

function TaskQueueStatus({ tasks }: TaskQueueStatusProps) {
  const counts = countByStatus(tasks)
  const active = tasks
    .filter((t) => t.status !== 'completed')
    .sort((a, b) => PRIORITY_ORDER[a.priority] - PRIORITY_ORDER[b.priority])

  return (
    <div className="task-queue">
      <div className="task-queue__summary">
        {STATUS_COUNTS.map((s) => (
          <span key={s} className={`task-queue__count task-queue__count--${s}`}>
            {counts[s]} {STATUS_LABEL[s]}
          </span>
        ))}
      </div>

      {active.length > 0 ? (
        <div className="task-queue__list">
          {active.map((t) => (
            <div key={t.id} className="task-queue__item">
              <span className={`task-queue__priority task-queue__priority--${t.priority}`}>
                {PRIORITY_MARKER[t.priority]}
              </span>
              <span className="task-queue__title">{t.title}</span>
              <span className={`task-queue__status task-queue__status--${t.status}`}>{t.status}</span>
              <span className="task-queue__agent">{t.assigned_agent ?? '--'}</span>
              {t.retry_count > 0 ? <span className="task-queue__retry">r:{t.retry_count}</span> : null}
              {t.depends_on.length > 0 ? <span className="task-queue__deps">dep:{t.depends_on.length}</span> : null}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  )
}

export { TaskQueueStatus }
export type { TaskQueueStatusProps }
