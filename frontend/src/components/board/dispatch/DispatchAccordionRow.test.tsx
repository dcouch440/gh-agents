import { describe, it, expect } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { DispatchAccordionRow } from './DispatchAccordionRow'
import type { DispatchEntry } from '@/stores/dispatchStore'

const makeEntry = (overrides: Partial<DispatchEntry> = {}): DispatchEntry => ({
  executionId: 'exec-1',
  stepId: 'step-1',
  status: 'completed',
  instruction: 'Configure this node',
  message: null,
  summary: 'Configured single-agent team',
  error: null,
  startedAt: '2026-01-01T00:00:00Z',
  trace: [
    { type: 'tool_start', toolName: 'set_node_name', toolId: 'tool-1', input: { name: 'Researcher' }, ts: '2026-01-01T00:00:01Z' },
    { type: 'tool_end', toolName: 'set_node_name', toolId: 'tool-1', result: 'ok', ts: '2026-01-01T00:00:02Z' },
  ],
  tokenBuffer: '',
  ...overrides,
})

describe('DispatchAccordionRow', () => {
  it('renders step name and status chip when entry exists', () => {
    render(
      <DispatchAccordionRow stepName="Research" instruction="Configure this node" entry={makeEntry()} />,
    )
    expect(screen.getByText('Research')).toBeInTheDocument()
    expect(screen.getByText('completed')).toBeInTheDocument()
  })

  it('shows waiting state when entry is null', () => {
    render(
      <DispatchAccordionRow stepName="Research" instruction="Configure this node" entry={null} />,
    )
    expect(screen.getByText('waiting...')).toBeInTheDocument()
  })

  it('shows instruction preview when collapsed', () => {
    render(
      <DispatchAccordionRow stepName="Research" instruction="Configure this new workflow node" entry={makeEntry()} />,
    )
    expect(screen.getByText(/Configure this new workflow node/)).toBeInTheDocument()
  })

  it('shows tool count when collapsed', () => {
    render(
      <DispatchAccordionRow stepName="Research" instruction="Configure" entry={makeEntry()} />,
    )
    expect(screen.getByText('1 tool(s)')).toBeInTheDocument()
  })

  it('shows running status chip for active dispatch', () => {
    render(
      <DispatchAccordionRow stepName="Research" instruction="Configure" entry={makeEntry({ status: 'running', summary: null })} />,
    )
    expect(screen.getByText('running')).toBeInTheDocument()
  })

  it('expands to show detail on click', () => {
    render(
      <DispatchAccordionRow stepName="Research" instruction="Configure this node" entry={makeEntry()} />,
    )

    // Click the header to expand
    fireEvent.click(screen.getByText('Research'))

    // Summary should be visible when expanded
    expect(screen.getByText('Configured single-agent team')).toBeInTheDocument()
  })

  it('shows the failure reason on a failed row without expanding it', () => {
    render(
      <DispatchAccordionRow
        stepName="Research"
        instruction="Configure this node"
        entry={makeEntry({ status: 'failed', summary: null, error: 'System node agent timed out after 120s' })}
      />,
    )
    expect(screen.getByText('System node agent timed out after 120s')).toBeInTheDocument()
  })

  it('shows the failure reason in full when expanded', () => {
    render(
      <DispatchAccordionRow
        stepName="Research"
        instruction="Configure this node"
        entry={makeEntry({ status: 'failed', summary: null, error: 'boom' })}
      />,
    )

    fireEvent.click(screen.getByText('Research'))

    expect(screen.getByText('boom')).toBeInTheDocument()
  })

  it('shows waiting message when expanded with null entry', () => {
    render(
      <DispatchAccordionRow stepName="Research" instruction="Configure" entry={null} />,
    )

    // Click the header to expand
    fireEvent.click(screen.getByText('Research'))

    expect(screen.getByText('Waiting for dispatch events...')).toBeInTheDocument()
  })
})
