import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen } from '@/test/render'

import { ExecutionStatusBadge } from './ExecutionStatusBadge'

describe('ExecutionStatusBadge', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns null for idle status', () => {
    const { container } = render(<ExecutionStatusBadge status="idle" />)
    expect(container.innerHTML).toBe('')
  })

  it('renders Running label for running status', () => {
    render(<ExecutionStatusBadge status="running" />)
    expect(screen.getByText('Running')).toBeInTheDocument()
  })

  it('renders Done label for completed status', () => {
    render(<ExecutionStatusBadge status="completed" />)
    expect(screen.getByText('Done')).toBeInTheDocument()
  })

  it('renders Failed label for failed status', () => {
    render(<ExecutionStatusBadge status="failed" />)
    expect(screen.getByText('Failed')).toBeInTheDocument()
  })

  it('renders Pending label for pending status', () => {
    render(<ExecutionStatusBadge status="pending" />)
    expect(screen.getByText('Pending')).toBeInTheDocument()
  })

  it('renders Paused label for paused status', () => {
    render(<ExecutionStatusBadge status="paused" />)
    expect(screen.getByText('Paused')).toBeInTheDocument()
  })

  it('renders Skipped label for skipped status', () => {
    render(<ExecutionStatusBadge status="skipped" />)
    expect(screen.getByText('Skipped')).toBeInTheDocument()
  })
})
