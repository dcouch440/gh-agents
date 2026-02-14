import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { ToolCheckList } from './ToolCheckList'
import type { Tool } from '@/types'

const tools: Tool[] = [
  { id: 'tool-1', name: 'SearchTool', description: 'Searches the web', user_id: null },
  { id: 'tool-2', name: 'CodeTool', description: 'Writes code', user_id: null },
  { id: 'tool-3', name: 'EmptyDesc', description: null, user_id: null },
]

const mockToggle = vi.fn()

beforeEach(() => {
  vi.clearAllMocks()
})

describe('ToolCheckList', () => {
  it('renders all tools with names', () => {
    render(<ToolCheckList tools={tools} selectedIds={new Set()} onToggle={mockToggle} disabled={false} />)
    expect(screen.getByText('SearchTool')).toBeInTheDocument()
    expect(screen.getByText('CodeTool')).toBeInTheDocument()
    expect(screen.getByText('EmptyDesc')).toBeInTheDocument()
  })

  it('renders tool descriptions when present', () => {
    render(<ToolCheckList tools={tools} selectedIds={new Set()} onToggle={mockToggle} disabled={false} />)
    expect(screen.getByText('Searches the web')).toBeInTheDocument()
    expect(screen.getByText('Writes code')).toBeInTheDocument()
  })

  it('checks selected tools', () => {
    render(<ToolCheckList tools={tools} selectedIds={new Set(['tool-1', 'tool-3'])} onToggle={mockToggle} disabled={false} />)
    const checkboxes = screen.getAllByRole('checkbox')
    expect(checkboxes[0]).toBeChecked()
    expect(checkboxes[1]).not.toBeChecked()
    expect(checkboxes[2]).toBeChecked()
  })

  it('calls onToggle when checkbox is clicked', async () => {
    const user = userEvent.setup()
    render(<ToolCheckList tools={tools} selectedIds={new Set()} onToggle={mockToggle} disabled={false} />)

    await user.click(screen.getByText('CodeTool'))
    expect(mockToggle).toHaveBeenCalledWith('tool-2')
  })

  it('disables all checkboxes when disabled is true', () => {
    render(<ToolCheckList tools={tools} selectedIds={new Set()} onToggle={mockToggle} disabled={true} />)
    const checkboxes = screen.getAllByRole('checkbox')
    for (const cb of checkboxes) {
      expect(cb).toBeDisabled()
    }
  })

  it('renders empty when tools array is empty', () => {
    const { container } = render(
      <ToolCheckList tools={[]} selectedIds={new Set()} onToggle={mockToggle} disabled={false} />,
    )
    expect(container.querySelectorAll('[role="checkbox"]')).toHaveLength(0)
  })
})
