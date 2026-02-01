import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ToolActivityFeed } from './ToolActivityFeed'
import type { ToolEvent } from './ToolActivityFeed'

const now = Date.now()

const events: ToolEvent[] = [
  { id: '1', toolName: 'search_files', status: 'completed', startedAt: now - 1200, completedAt: now, detail: 'done' },
  { id: '2', toolName: 'read_file', status: 'running', startedAt: now - 400, completedAt: null, detail: 'reading...' },
]

describe('ToolActivityFeed', () => {
  it('renders all tool events', () => {
    render(<ToolActivityFeed events={events} now={now} />)
    expect(screen.getByText('search_files')).toBeInTheDocument()
    expect(screen.getByText('read_file')).toBeInTheDocument()
  })

  it('shows hint when provided', () => {
    render(<ToolActivityFeed events={events} hint="Atlas is working..." now={now} />)
    expect(screen.getByText('Atlas is working...')).toBeInTheDocument()
  })

  it('hides hint when null', () => {
    const { container } = render(<ToolActivityFeed events={events} hint={null} now={now} />)
    expect(container.querySelector('.tool-feed__hint')).not.toBeInTheDocument()
  })

  it('renders empty when no events', () => {
    const { container } = render(<ToolActivityFeed events={[]} now={now} />)
    expect(container.querySelector('.tool-box')).not.toBeInTheDocument()
  })
})
