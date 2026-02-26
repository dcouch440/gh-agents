import { describe, it, expect } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { ActivityTimeline } from './ActivityTimeline'
import type { ActivityEntry } from '@/stores/activity'
import { ACTIVITY } from '@/types/activity'

const makeEntry = (seq: number, overrides: Partial<ActivityEntry> = {}): ActivityEntry => ({
  id: `act_${seq}`,
  seq,
  event: { type: ACTIVITY.WORKFLOW_STEP_STARTED, workflowId: 'wf-1', stepId: `s-${seq}`, stepName: `Step ${seq}`, agentId: null, executionId: null },
  ts: new Date().toISOString(),
  runId: 'run-1',
  userId: null,
  receivedAt: Date.now(),
  ...overrides,
})

describe('ActivityTimeline', () => {
  it('returns null when activities is empty', () => {
    const { container } = render(<ActivityTimeline activities={[]} />)
    expect(container.innerHTML).toBe('')
  })

  it('shows event count', () => {
    const entries = Array.from({ length: 3 }, (_, i) => makeEntry(i + 1))
    render(<ActivityTimeline activities={entries} />)
    expect(screen.getByText('3 event(s)')).toBeInTheDocument()
  })

  it('shows last 5 events when collapsed with more than 5', () => {
    const entries = Array.from({ length: 8 }, (_, i) => makeEntry(i + 1))
    render(<ActivityTimeline activities={entries} />)

    // Should see step names for last 5 entries (4-8)
    expect(screen.queryByText(/Step 1/)).not.toBeInTheDocument()
    expect(screen.queryByText(/Step 2/)).not.toBeInTheDocument()
    expect(screen.queryByText(/Step 3/)).not.toBeInTheDocument()
    expect(screen.getByText(/Step 4/)).toBeInTheDocument()
    expect(screen.getByText(/Step 8/)).toBeInTheDocument()
  })

  it('shows all events when expanded', () => {
    const entries = Array.from({ length: 8 }, (_, i) => makeEntry(i + 1))
    render(<ActivityTimeline activities={entries} />)

    // Click to expand
    fireEvent.click(screen.getByText('Activity'))

    expect(screen.getByText(/Step 1/)).toBeInTheDocument()
    expect(screen.getByText(/Step 8/)).toBeInTheDocument()
  })
})
