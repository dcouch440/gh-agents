import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { FeedStream } from './FeedStream'
import type { FeedItem } from '@/types'

const items: FeedItem[] = [
  {
    id: 'f1',
    agent_id: 'a1',
    content: 'Started search',
    item_type: 'task_started',
    verbosity_level: 'normal',
    timestamp: '2026-01-31T10:00:00Z',
  },
  {
    id: 'f2',
    agent_id: 'a1',
    content: 'Found 3 files',
    item_type: 'agent_report',
    verbosity_level: 'normal',
    timestamp: '2026-01-31T10:00:05Z',
  },
  {
    id: 'f3',
    agent_id: 'a2',
    content: 'Auth module updated',
    item_type: 'milestone',
    verbosity_level: 'normal',
    timestamp: '2026-01-31T10:00:10Z',
  },
]

describe('FeedStream', () => {
  it('renders feed items', () => {
    render(<FeedStream items={items} maxVisible={10} />)
    expect(screen.getByText('Started search')).toBeInTheDocument()
    expect(screen.getByText('Found 3 files')).toBeInTheDocument()
    expect(screen.getByText('Auth module updated')).toBeInTheDocument()
  })

  it('respects maxVisible limit', () => {
    render(<FeedStream items={items} maxVisible={2} />)
    expect(screen.queryByText('Started search')).not.toBeInTheDocument()
    expect(screen.getByText('Found 3 files')).toBeInTheDocument()
    expect(screen.getByText('Auth module updated')).toBeInTheDocument()
  })

  it('shows most recent items when limited', () => {
    render(<FeedStream items={items} maxVisible={1} />)
    expect(screen.getByText('Auth module updated')).toBeInTheDocument()
    expect(screen.queryByText('Started search')).not.toBeInTheDocument()
  })

  it('renders type icons', () => {
    render(<FeedStream items={items} maxVisible={10} />)
    expect(screen.getByText('+')).toBeInTheDocument()
    expect(screen.getByText('>')).toBeInTheDocument()
    expect(screen.getByText('@')).toBeInTheDocument()
  })
})
