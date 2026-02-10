import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { HistoricalRunSummary } from './HistoricalRunSummary'
import type { WorkflowExecutionSummary } from '@/types'

vi.mock('@/components/primitives', () => ({
  StatusBadge: ({ label }: { label: string }) => <span data-testid="badge">{label}</span>,
  MarkdownPreview: ({ content }: { content: string }) => <div data-testid="markdown">{content}</div>,
}))

const makeRun = (overrides: Partial<WorkflowExecutionSummary> = {}): WorkflowExecutionSummary => ({
  id: 'run-1',
  workflow_id: 'wf-1',
  status: 'completed',
  started_at: '2025-01-01T00:00:00Z',
  completed_at: '2025-01-01T00:00:10Z',
  outputs: null,
  error: null,
  ...overrides,
})

describe('HistoricalRunSummary', () => {
  it('renders completed run with duration', () => {
    render(<HistoricalRunSummary run={makeRun()} />)
    expect(screen.getByTestId('badge')).toHaveTextContent('Completed')
    expect(screen.getByText('10.0s')).toBeInTheDocument()
  })

  it('renders failed run with error message', () => {
    render(<HistoricalRunSummary run={makeRun({ status: 'failed', error: 'LLM timeout' })} />)
    expect(screen.getByTestId('badge')).toHaveTextContent('Failed')
    expect(screen.getByText('LLM timeout')).toBeInTheDocument()
  })

  it('renders outputs with response field', () => {
    render(<HistoricalRunSummary run={makeRun({ outputs: { '': { response: 'Hello world' } } })} />)
    expect(screen.getByTestId('markdown')).toBeInTheDocument()
  })

  it('shows no output message when outputs are null', () => {
    render(<HistoricalRunSummary run={makeRun({ outputs: null })} />)
    expect(screen.getByText('No output data')).toBeInTheDocument()
  })

  it('shows no output message when outputs are empty', () => {
    render(<HistoricalRunSummary run={makeRun({ outputs: {} })} />)
    expect(screen.getByText('No output data')).toBeInTheDocument()
  })
})
