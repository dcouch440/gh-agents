import { render, screen } from '@testing-library/react'
import { ToolIndicator } from './ToolIndicator'
import { TOOL_LABELS, getToolLabel } from './toolLabels'
import type { ToolStatus } from '@/types'

describe('ToolIndicator', () => {
  describe('tool variant', () => {
    it('renders running state with label', () => {
      render(<ToolIndicator variant="tool" toolName="update_prompt" status="running" />)
      expect(screen.getByText('Updating prompt...')).toBeInTheDocument()
    })

    it('renders complete state with label', () => {
      render(<ToolIndicator variant="tool" toolName="update_prompt" status="complete" />)
      expect(screen.getByText('Updated prompt')).toBeInTheDocument()
    })

    it('renders all tool names with running labels', () => {
      const expected: Record<string, string> = {
        update_prompt: 'Updating prompt...',
        read_context: 'Reading context...',
        think: 'Thinking...',
        render_panel: 'Rendering panel...',
      }

      for (const [toolName, label] of Object.entries(expected)) {
        const { unmount } = render(
          <ToolIndicator variant="tool" toolName={toolName} status="running" />,
        )
        expect(screen.getByText(label)).toBeInTheDocument()
        unmount()
      }
    })

    it('renders all tool names with complete labels', () => {
      const expected: Record<string, string> = {
        update_prompt: 'Updated prompt',
        read_context: 'Read context',
        think: 'Thought',
        render_panel: 'Rendered panel',
      }

      for (const [toolName, label] of Object.entries(expected)) {
        const { unmount } = render(
          <ToolIndicator variant="tool" toolName={toolName} status="complete" />,
        )
        expect(screen.getByText(label)).toBeInTheDocument()
        unmount()
      }
    })

    it('falls back gracefully for unknown tool names', () => {
      render(<ToolIndicator variant="tool" toolName="some_custom_tool" status="running" />)
      expect(screen.getByText('some custom tool...')).toBeInTheDocument()
    })

    it('falls back gracefully for unknown tool names when complete', () => {
      render(<ToolIndicator variant="tool" toolName="some_custom_tool" status="complete" />)
      expect(screen.getByText('some custom tool')).toBeInTheDocument()
    })
  })

  describe('doc_update variant', () => {
    it('renders with document title', () => {
      render(<ToolIndicator variant="doc_update" title="API Reference" />)
      expect(screen.getByText(/Updated/)).toBeInTheDocument()
      expect(screen.getByText(/API Reference/)).toBeInTheDocument()
    })
  })

  describe('TOOL_LABELS', () => {
    it('contains exactly 4 tool mappings', () => {
      expect(Object.keys(TOOL_LABELS)).toHaveLength(4)
    })
  })

  describe('getToolLabel', () => {
    it('returns mapped label for known tools', () => {
      expect(getToolLabel('update_prompt', 'running')).toBe('Updating prompt...')
      expect(getToolLabel('think', 'complete')).toBe('Thought')
    })

    it('returns humanized name for unknown tools', () => {
      expect(getToolLabel('unknown_tool', 'running')).toBe('unknown tool...')
      expect(getToolLabel('unknown_tool', 'complete')).toBe('unknown tool')
    })

    it('handles each status correctly', () => {
      const statuses: ToolStatus[] = ['running', 'complete']
      for (const status of statuses) {
        const label = getToolLabel('read_context', status)
        expect(typeof label).toBe('string')
        expect(label.length).toBeGreaterThan(0)
      }
    })
  })
})
