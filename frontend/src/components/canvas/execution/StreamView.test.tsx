import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'

import { StreamView } from './StreamView'

describe('StreamView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns null when content is empty and status is idle', () => {
    const { container } = render(<StreamView content="" status="idle" />)
    expect(container.innerHTML).toBe('')
  })

  it('renders content text', () => {
    render(<StreamView content="Hello world" status="running" />)
    expect(screen.getByText('Hello world', { exact: false })).toBeInTheDocument()
  })

  it('shows blinking cursor when running', () => {
    const { container } = render(<StreamView content="test" status="running" />)
    expect(container.innerHTML).toContain('\u258C')
  })

  it('does not show cursor when completed', () => {
    const { container } = render(<StreamView content="done" status="completed" />)
    expect(container.innerHTML).not.toContain('\u258C')
  })

  it('shows error strip when error provided', () => {
    render(
      <StreamView content="text" status="failed" error="Rate limit" />
    )
    expect(screen.getByText('Rate limit')).toBeInTheDocument()
  })

  it('renders content even when empty string with running status', () => {
    const { container } = render(<StreamView content="" status="running" />)
    expect(container.innerHTML).toContain('\u258C')
  })
})
