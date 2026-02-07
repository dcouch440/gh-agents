import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ReviewCard } from './ReviewCard'
import { mockAgentExecution } from '@/test/fixtures'

describe('ReviewCard', () => {
  it('renders execution input text', () => {
    render(
      <ReviewCard execution={mockAgentExecution} selected={false} onSelect={vi.fn()} />,
    )
    const matches = screen.getAllByText(/Please review the following changes/)
    expect(matches.length).toBeGreaterThanOrEqual(1)
  })

  it('renders execution status badge', () => {
    render(
      <ReviewCard execution={mockAgentExecution} selected={false} onSelect={vi.fn()} />,
    )
    expect(screen.getByText('Awaiting Review')).toBeInTheDocument()
  })

  it('renders time ago', () => {
    render(
      <ReviewCard execution={mockAgentExecution} selected={false} onSelect={vi.fn()} />,
    )
    expect(screen.getByText(/ago/)).toBeInTheDocument()
  })

  it('calls onSelect with execution id when clicked', async () => {
    const user = userEvent.setup()
    const onSelect = vi.fn()
    const { container } = render(
      <ReviewCard execution={mockAgentExecution} selected={false} onSelect={onSelect} />,
    )

    await user.click(container.firstChild as HTMLElement)
    expect(onSelect).toHaveBeenCalledWith('exec-001')
  })

  it('renders without error when selected', () => {
    const { container } = render(
      <ReviewCard execution={mockAgentExecution} selected={true} onSelect={vi.fn()} />,
    )
    expect(container.firstChild).toBeInTheDocument()
  })
})
