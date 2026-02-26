import { describe, it, expect } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { AgentTraceCard } from './AgentTraceCard'
import type { AgentTrace } from '@/stores/agentTraceStore'

const makeTrace = (overrides: Partial<AgentTrace> = {}): AgentTrace => ({
  agentExecutionId: 'exec-1',
  agentName: 'Research Agent',
  stepId: 'step-1',
  events: [],
  ...overrides,
})

describe('AgentTraceCard', () => {
  it('renders collapsed agent name', () => {
    render(<AgentTraceCard trace={makeTrace()} />)
    expect(screen.getByText('Research Agent')).toBeInTheDocument()
  })

  it('falls back to truncated execution ID when no agent name', () => {
    render(<AgentTraceCard trace={makeTrace({ agentName: null, agentExecutionId: 'abcdef12-9999' })} />)
    expect(screen.getByText('abcdef12')).toBeInTheDocument()
  })

  it('shows tool count badge', () => {
    render(
      <AgentTraceCard
        trace={makeTrace({
          events: [
            { type: 'tool_call', toolName: 'search', toolId: 't1', input: { q: 'test' }, ts: '' },
            { type: 'tool_result', toolName: 'search', toolId: 't1', result: 'found', ts: '' },
            { type: 'tool_call', toolName: 'write', toolId: 't2', input: { text: 'hi' }, ts: '' },
          ],
        })}
      />,
    )
    expect(screen.getByText('2 tool(s)')).toBeInTheDocument()
  })

  it('expands on click showing system prompt section', () => {
    render(
      <AgentTraceCard
        trace={makeTrace({
          events: [
            { type: 'system_prompt', content: 'You are a research assistant', ts: '' },
          ],
        })}
      />,
    )

    // Click the header to expand
    fireEvent.click(screen.getByText('Research Agent'))

    expect(screen.getByText('System Prompt')).toBeInTheDocument()
  })

  it('renders ToolCallCard for tool_call events with paired result', () => {
    render(
      <AgentTraceCard
        trace={makeTrace({
          events: [
            { type: 'tool_call', toolName: 'web_search', toolId: 't1', input: { query: 'AI trends' }, ts: '' },
            { type: 'tool_result', toolName: 'web_search', toolId: 't1', result: 'Found 10 results', ts: '' },
          ],
        })}
      />,
    )

    // Expand the card
    fireEvent.click(screen.getByText('Research Agent'))

    // ToolCallCard renders the input key via VS Code-style coloring
    expect(screen.getByText('query')).toBeInTheDocument()
  })
})
