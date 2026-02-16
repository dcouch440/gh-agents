import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ExecutionTimelineEntry } from './ExecutionTimelineEntry'
import type { StepExecutionState } from '@/stores'

vi.mock('@/components/primitives', () => ({
  StatusBadge: ({ label }: { label: string }) => <span data-testid="badge">{label}</span>,
  MarkdownPreview: ({ content }: { content: string }) => <div data-testid="markdown">{content}</div>,
}))

const makeStep = (overrides: Partial<StepExecutionState> = {}): StepExecutionState => ({
  status: 'success',
  stepName: 'Test Step',
  agentId: null,
  executionId: null,
  output: 'result text',
  error: null,
  inputTokens: 100,
  outputTokens: 50,
  durationMs: 1500,
  forEachProgress: null,
  subWorkflowProgress: null,
  startedAt: '2025-01-01T00:00:00Z',
  completedAt: '2025-01-01T00:00:01Z',
  ...overrides,
})

describe('ExecutionTimelineEntry', () => {
  it('renders step name and status badge in collapsed state', () => {
    render(<ExecutionTimelineEntry stepId="s1" stepState={makeStep()} isLast={false} />)
    expect(screen.getByText('Test Step')).toBeInTheDocument()
    expect(screen.getByTestId('badge')).toHaveTextContent('Completed')
  })

  it('shows metrics when available', () => {
    render(<ExecutionTimelineEntry stepId="s1" stepState={makeStep()} isLast={false} />)
    expect(screen.getByText('1.5s')).toBeInTheDocument()
    expect(screen.getByText('100 in / 50 out')).toBeInTheDocument()
  })

  it('expands on click to show output', async () => {
    const user = userEvent.setup()
    render(<ExecutionTimelineEntry stepId="s1" stepState={makeStep()} isLast={false} />)

    expect(screen.queryByTestId('markdown')).not.toBeInTheDocument()

    await user.click(screen.getByText('Test Step'))

    expect(screen.getByTestId('markdown')).toHaveTextContent('result text')
  })

  it('shows error message when step failed', () => {
    render(<ExecutionTimelineEntry stepId="s1" stepState={makeStep({ status: 'error', error: 'timeout', output: null })} isLast={false} />)
    expect(screen.getByTestId('badge')).toHaveTextContent('Failed')
  })

  it('shows for-each progress when present', () => {
    render(<ExecutionTimelineEntry stepId="s1" stepState={makeStep({ forEachProgress: { completed: 3, total: 10 } })} isLast={false} />)
    expect(screen.getByText('3/10 items')).toBeInTheDocument()
  })

  it('shows running status badge', () => {
    render(
      <ExecutionTimelineEntry
        stepId="s1"
        stepState={makeStep({ status: 'running', output: null, completedAt: null, durationMs: null })}
        isLast={false}
      />,
    )
    expect(screen.getByTestId('badge')).toHaveTextContent('Running')
  })

  it('falls back to stepId when stepName is null', () => {
    render(<ExecutionTimelineEntry stepId="step-uuid-123" stepState={makeStep({ stepName: null })} isLast={false} />)
    expect(screen.getByText('step-uuid-123')).toBeInTheDocument()
  })

  it('shows sub-workflow progress when subWorkflowProgress is present', () => {
    render(
      <ExecutionTimelineEntry
        stepId="s1"
        stepState={makeStep({
          subWorkflowProgress: {
            childExecutionId: 'ce1',
            totalSteps: 3,
            completedSteps: 2,
            status: 'running',
            childSteps: [
              { childStepId: 'cs1', childStepName: 'Designer', status: 'success', inputTokens: 100, outputTokens: 50, durationMs: 1000, error: null },
              { childStepId: 'cs2', childStepName: 'Agent 1', status: 'running', inputTokens: null, outputTokens: null, durationMs: null, error: null },
            ],
          },
        })}
        isLast={false}
      />,
    )
    expect(screen.getByText('2/3 child steps')).toBeInTheDocument()
    expect(screen.getByText('Designer')).toBeInTheDocument()
    expect(screen.getByText('Agent 1')).toBeInTheDocument()
  })

  it('does not show child timeline when subWorkflowProgress is null', () => {
    render(<ExecutionTimelineEntry stepId="s1" stepState={makeStep()} isLast={false} />)
    expect(screen.queryByText('Sub-workflow')).not.toBeInTheDocument()
  })

  it('does not show metrics row when none available', () => {
    render(
      <ExecutionTimelineEntry
        stepId="s1"
        stepState={makeStep({ durationMs: null, inputTokens: null, outputTokens: null, forEachProgress: null })}
        isLast={true}
      />,
    )
    // Should not have the metrics row — no duration, tokens, or forEach
    expect(screen.queryByText('items')).not.toBeInTheDocument()
  })
})
