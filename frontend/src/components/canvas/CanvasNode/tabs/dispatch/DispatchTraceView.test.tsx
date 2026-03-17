import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { DispatchTraceView } from './DispatchTraceView'
import type { DispatchEntry } from '@/stores/dispatchStore'

vi.mock('@/components/primitives', () => ({
  TerminalBlock: ({ content }: { content: string }) => <pre data-testid="terminal">{content}</pre>,
}))

vi.mock('./ToolCallCard', () => ({
  ToolCallCard: ({ toolName, status }: { toolName: string; status: string }) => (
    <div data-testid={`tool-card-${toolName}`}>{status}</div>
  ),
}))

const makeEntry = (overrides: Partial<DispatchEntry> = {}): DispatchEntry => ({
  executionId: 'exec-1',
  stepId: 'step-1',
  status: 'running',
  instruction: 'Do something',
  message: null,
  summary: null,
  error: null,
  startedAt: '2025-01-01T00:00:00Z',
  trace: [],
  tokenBuffer: '',
  ...overrides,
})

describe('DispatchTraceView', () => {
  it('renders text segments with TerminalBlock', () => {
    // Text segments before the first tool call are hidden (builder preamble filter),
    // so include a tool call first to make the text segment visible.
    const entry = makeEntry({
      trace: [
        { type: 'tool_start', toolName: 'think', toolId: 't0', input: {}, ts: '2025-01-01T00:00:00Z' },
        { type: 'tool_end', toolName: 'think', toolId: 't0', result: {}, ts: '2025-01-01T00:00:01Z' },
        { type: 'token', content: 'Hello world', ts: '2025-01-01T00:00:02Z' },
      ],
    })
    render(<DispatchTraceView entry={entry} />)
    expect(screen.getByTestId('terminal')).toHaveTextContent('Hello world')
  })

  it('renders tool call cards', () => {
    const entry = makeEntry({
      trace: [
        { type: 'tool_start', toolName: 'web_search', toolId: 't1', input: { q: 'test' }, ts: '2025-01-01T00:00:00Z' },
        { type: 'tool_end', toolName: 'web_search', toolId: 't1', result: { ok: true }, ts: '2025-01-01T00:00:01Z' },
      ],
    })
    render(<DispatchTraceView entry={entry} />)
    expect(screen.getByTestId('tool-card-web_search')).toHaveTextContent('complete')
  })

  it('renders error segments', () => {
    const entry = makeEntry({
      trace: [
        { type: 'error', error: 'Something failed', ts: '2025-01-01T00:00:00Z' },
      ],
    })
    render(<DispatchTraceView entry={entry} />)
    expect(screen.getByText('Something failed')).toBeInTheDocument()
  })

  it('shows blinking cursor when running', () => {
    const entry = makeEntry({ status: 'running' })
    const { container } = render(<DispatchTraceView entry={entry} />)
    expect(container.textContent).toContain('\u258C')
  })

  it('hides cursor when completed', () => {
    const entry = makeEntry({ status: 'completed' })
    const { container } = render(<DispatchTraceView entry={entry} />)
    expect(container.textContent).not.toContain('\u258C')
  })
})
