import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { TaskQueueStatus } from './TaskQueueStatus'
import type { Task } from '@/types'

const tasks: Task[] = [
  { id: 't1', slice_id: null, title: 'Fix auth bug', description: '', assigned_tier: 'worker', assigned_agent: 'Forge', status: 'in_progress', priority: 'high', context_files: [], metadata: null, depends_on: [], retry_count: 1, max_retries: 3, last_error: null, created_at: '', updated_at: '' },
  { id: 't2', slice_id: null, title: 'Add tests', description: '', assigned_tier: 'utility', assigned_agent: null, status: 'pending', priority: 'normal', context_files: [], metadata: null, depends_on: ['t1'], retry_count: 0, max_retries: 3, last_error: null, created_at: '', updated_at: '' },
  { id: 't3', slice_id: null, title: 'Deploy', description: '', assigned_tier: 'orchestrator', assigned_agent: null, status: 'completed', priority: 'low', context_files: [], metadata: null, depends_on: [], retry_count: 0, max_retries: 3, last_error: null, created_at: '', updated_at: '' },
]

describe('TaskQueueStatus', () => {
  it('renders summary counts', () => {
    render(<TaskQueueStatus tasks={tasks} />)
    expect(screen.getByText('1 pending')).toBeInTheDocument()
    expect(screen.getByText('1 active')).toBeInTheDocument()
    expect(screen.getByText('1 done')).toBeInTheDocument()
  })

  it('shows active tasks sorted by priority', () => {
    const { container } = render(<TaskQueueStatus tasks={tasks} />)
    const items = container.querySelectorAll('.task-queue__item')
    expect(items.length).toBe(2)
    expect(items[0]?.textContent).toContain('Fix auth bug')
    expect(items[1]?.textContent).toContain('Add tests')
  })

  it('shows retry count', () => {
    render(<TaskQueueStatus tasks={tasks} />)
    expect(screen.getByText('r:1')).toBeInTheDocument()
  })

  it('shows dependency count', () => {
    render(<TaskQueueStatus tasks={tasks} />)
    expect(screen.getByText('dep:1')).toBeInTheDocument()
  })

  it('shows agent or dashes', () => {
    render(<TaskQueueStatus tasks={tasks} />)
    expect(screen.getByText('Forge')).toBeInTheDocument()
    expect(screen.getByText('--')).toBeInTheDocument()
  })
})
