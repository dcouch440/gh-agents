import { Box, Typography } from '@mui/material'
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

const COUNT_COLOR: Record<TaskStatus, string> = {
  pending: 'text.disabled',
  in_progress: 'text.primary',
  review: 'text.secondary',
  completed: 'text.secondary',
  failed: 'error.main',
}

const PRIORITY_COLOR: Record<TaskPriority, string> = {
  urgent: 'error.main',
  high: 'warning.main',
  normal: 'text.disabled',
  low: 'text.disabled',
}

const STATUS_COLOR: Record<TaskStatus, string> = {
  pending: 'text.disabled',
  in_progress: 'text.secondary',
  review: 'info.main',
  completed: 'text.secondary',
  failed: 'error.main',
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
    <Box sx={{ fontSize: '0.75rem', lineHeight: 1.4 }}>
      <Box sx={{ display: 'flex', gap: 2, color: 'text.secondary', mb: '2px' }}>
        {STATUS_COUNTS.map((s) => (
          <Typography
            key={s}
            component="span"
            sx={{
              fontSize: 'inherit',
              lineHeight: 'inherit',
              color: COUNT_COLOR[s],
            }}
          >
            {counts[s]} {STATUS_LABEL[s]}
          </Typography>
        ))}
      </Box>

      {active.length > 0 ? (
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0 }}>
          {active.map((t) => (
            <Box
              key={t.id}
              sx={{
                display: 'flex',
                gap: 1,
                py: '1px',
                alignItems: 'baseline',
              }}
            >
              <Typography
                component="span"
                sx={{
                  fontSize: 'inherit',
                  lineHeight: 'inherit',
                  flexShrink: 0,
                  width: '4ch',
                  textAlign: 'right',
                  color: PRIORITY_COLOR[t.priority],
                }}
              >
                {PRIORITY_MARKER[t.priority]}
              </Typography>
              <Typography
                component="span"
                sx={{
                  fontSize: 'inherit',
                  lineHeight: 'inherit',
                  flex: 1,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                  color: 'text.primary',
                }}
              >
                {t.title}
              </Typography>
              <Typography
                component="span"
                sx={{
                  fontSize: 'inherit',
                  lineHeight: 'inherit',
                  flexShrink: 0,
                  color: STATUS_COLOR[t.status],
                }}
              >
                {t.status}
              </Typography>
              <Typography
                component="span"
                sx={{
                  fontSize: 'inherit',
                  lineHeight: 'inherit',
                  flexShrink: 0,
                  color: 'text.disabled',
                  width: '8ch',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                }}
              >
                {t.assigned_agent ?? '--'}
              </Typography>
              {t.retry_count > 0 ? (
                <Typography
                  component="span"
                  sx={{
                    fontSize: 'inherit',
                    lineHeight: 'inherit',
                    flexShrink: 0,
                    color: 'text.disabled',
                  }}
                >
                  r:{t.retry_count}
                </Typography>
              ) : null}
              {t.depends_on.length > 0 ? (
                <Typography
                  component="span"
                  sx={{
                    fontSize: 'inherit',
                    lineHeight: 'inherit',
                    flexShrink: 0,
                    color: 'text.disabled',
                  }}
                >
                  dep:{t.depends_on.length}
                </Typography>
              ) : null}
            </Box>
          ))}
        </Box>
      ) : null}
    </Box>
  )
}

export { TaskQueueStatus }
export type { TaskQueueStatusProps }
