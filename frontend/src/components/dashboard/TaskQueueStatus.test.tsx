import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { TaskQueueStatus } from './TaskQueueStatus'
import type { Task } from '@/types'

const tasks: Task[] = [
  { id: 't1', slice_id: null, title: 'Fix auth bug', description: '', assigned_agent: 'Forge', status: 'in_progress', priority: 'high', context_files: [], metadata: null, depends_on: [], retry_count: 1, max_retries: 3, last_error: null, created_at: '', updated_at: '' },
  { id: 't2', slice_id: null, title: 'Add tests', description: '', assigned_agent: null, status: 'pending', priority: 'normal', context_files: [], metadata: null, depends_on: ['t1'], retry_count: 0, max_retries: 3, last_error: null, created_at: '', updated_at: '' },
  { id: 't3', slice_id: null, title: 'Deploy', description: '', assigned_agent: null, status: 'completed', priority: 'low', context_files: [], metadata: null, depends_on: [], retry_count: 0, max_retries: 3, last_error: null, created_at: '', updated_at: '' },
]

describe('TaskQueueStatus', () => {
  it('renders summary counts', () => {
    render(<TaskQueueStatus tasks={tasks} />)
    expect(screen.getByText('1 pending')).toBeInTheDocument()
    expect(screen.getByText('1 active')).toBeInTheDocument()
    expect(screen.getByText('1 done')).toBeInTheDocument()
  })

  it('shows active tasks sorted by priority', () => {
    render(<TaskQueueStatus tasks={tasks} />)
    const titles = screen.getAllByText(/Fix auth bug|Add tests/)
    expect(titles).toHaveLength(2)
    // high priority 'Fix auth bug' should come before normal priority 'Add tests'
    const allText = document.body.textContent ?? '' // eslint-disable-line @typescript-eslint/no-unnecessary-condition -- textContent is nullable per DOM spec
    const fixIndex = allText.indexOf('Fix auth bug')
    const addIndex = allText.indexOf('Add tests')
    expect(fixIndex).toBeLessThan(addIndex)
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
