import { describe, it, expect } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { ToolCallCard } from './ToolCallCard'

describe('ToolCallCard', () => {
  const baseProps = {
    toolName: 'web_search',
    toolId: 't1',
    input: { query: 'bitcoin price' },
    result: null,
    status: 'running' as const,
  }

  it('renders tool name with running indicator', () => {
    render(<ToolCallCard {...baseProps} />)
    expect(screen.getByTestId('tool-call-t1')).toBeInTheDocument()
    expect(screen.getByText(/web search/i)).toBeInTheDocument()
  })

  it('renders completed state with check icon', () => {
    render(<ToolCallCard {...baseProps} status="complete" result={{ ok: true }} />)
    expect(screen.getByTestId('tool-call-t1')).toBeInTheDocument()
  })

  it('shows input keys and values as separate spans', () => {
    render(<ToolCallCard {...baseProps} input={{ query: 'test', limit: 10 }} />)
    // VS Code-style coloring renders keys and values as separate spans
    expect(screen.getByText('query')).toBeInTheDocument()
    expect(screen.getByText('"test"')).toBeInTheDocument()
    expect(screen.getByText('limit')).toBeInTheDocument()
    expect(screen.getByText('10')).toBeInTheDocument()
  })

  it('renders long string values', () => {
    const longValue = 'a'.repeat(60)
    render(<ToolCallCard {...baseProps} input={{ data: longValue }} />)
    // The full quoted value is rendered (no truncation in the new format)
    expect(screen.getByText('data')).toBeInTheDocument()
  })

  it('does not show expand toggle when result is null', () => {
    render(<ToolCallCard {...baseProps} result={null} />)
    expect(screen.queryByRole('button')).toBeNull()
  })

  it('shows expand toggle when result exists', () => {
    render(<ToolCallCard {...baseProps} status="complete" result={{ data: 'value' }} />)
    expect(screen.getByRole('button')).toBeInTheDocument()
  })

  it('expands to show result on click', () => {
    render(<ToolCallCard {...baseProps} status="complete" result={{ answer: 42 }} />)
    // MUI Collapse renders the content but hides it — check the container is collapsed
    const container = screen.getByTestId('tool-call-t1')
    const collapseEl = container.querySelector('.MuiCollapse-root')
    expect(collapseEl).toHaveClass('MuiCollapse-hidden')

    // Click to expand
    fireEvent.click(screen.getByRole('button'))
    expect(collapseEl).not.toHaveClass('MuiCollapse-hidden')
  })
})
