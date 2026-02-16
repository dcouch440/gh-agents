import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ChildStepTimeline } from './ChildStepTimeline'
import type { SubWorkflowProgress } from '@/stores'

vi.mock('@/components/primitives', () => ({
  StatusBadge: ({ label }: { label: string }) => <span data-testid="badge">{label}</span>,
}))

const makeProgress = (overrides: Partial<SubWorkflowProgress> = {}): SubWorkflowProgress => ({
  childExecutionId: 'ce-1',
  totalSteps: 3,
  completedSteps: 1,
  status: 'running',
  childSteps: [
    { childStepId: 'cs1', childStepName: 'Designer', status: 'success', inputTokens: 200, outputTokens: 100, durationMs: 2000, error: null },
    { childStepId: 'cs2', childStepName: 'Agent 1', status: 'running', inputTokens: null, outputTokens: null, durationMs: null, error: null },
  ],
  ...overrides,
})

describe('ChildStepTimeline', () => {
  it('renders header with progress count', () => {
    render(<ChildStepTimeline progress={makeProgress()} />)
    expect(screen.getByText('Sub-workflow')).toBeInTheDocument()
    expect(screen.getByText('1/3 steps')).toBeInTheDocument()
  })

  it('renders child step names', () => {
    render(<ChildStepTimeline progress={makeProgress()} />)
    expect(screen.getByText('Designer')).toBeInTheDocument()
    expect(screen.getByText('Agent 1')).toBeInTheDocument()
  })

  it('renders status badges for each child step', () => {
    render(<ChildStepTimeline progress={makeProgress()} />)
    const badges = screen.getAllByTestId('badge')
    expect(badges).toHaveLength(2)
    expect(badges[0]).toHaveTextContent('Completed')
    expect(badges[1]).toHaveTextContent('Running')
  })

  it('shows metrics when available', () => {
    render(<ChildStepTimeline progress={makeProgress()} />)
    expect(screen.getByText('2.0s')).toBeInTheDocument()
    expect(screen.getByText('200 in / 100 out')).toBeInTheDocument()
  })

  it('shows error for failed child step', () => {
    render(
      <ChildStepTimeline
        progress={makeProgress({
          childSteps: [
            { childStepId: 'cs1', childStepName: 'Agent 1', status: 'error', inputTokens: null, outputTokens: null, durationMs: null, error: 'LLM timeout' },
          ],
        })}
      />,
    )
    expect(screen.getByText('LLM timeout')).toBeInTheDocument()
  })

  it('shows failed badge when overall status is failed', () => {
    render(<ChildStepTimeline progress={makeProgress({ status: 'failed' })} />)
    const badges = screen.getAllByTestId('badge')
    // First badge is the overall "Failed" badge, then per-child badges
    expect(badges[0]).toHaveTextContent('Failed')
  })
})
