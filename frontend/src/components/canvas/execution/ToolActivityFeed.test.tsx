import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'

import { ToolActivityFeed } from './ToolActivityFeed'
import type { StreamToolUse } from '@/stores/stepStreamStore'

const makeTool = (
  overrides: Partial<StreamToolUse> & { toolName: string; toolId: string }
): StreamToolUse => ({
  status: 'running',
  startedAt: '2025-01-01T00:00:00Z',
  ...overrides,
})

describe('ToolActivityFeed', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns null for empty tools array', () => {
    const { container } = render(<ToolActivityFeed tools={[]} />)
    expect(container.innerHTML).toBe('')
  })

  it('renders tool names in full mode', () => {
    const tools: StreamToolUse[] = [
      makeTool({ toolName: 'write_file', toolId: 't1', status: 'running' }),
      makeTool({ toolName: 'read_file', toolId: 't2', status: 'completed' }),
    ]

    render(<ToolActivityFeed tools={tools} />)

    expect(screen.getByText('write_file')).toBeInTheDocument()
    expect(screen.getByText('read_file')).toBeInTheDocument()
  })

  it('renders tool names in compact mode', () => {
    const tools: StreamToolUse[] = [
      makeTool({ toolName: 'write_file', toolId: 't1', status: 'running' }),
      makeTool({ toolName: 'read_file', toolId: 't2', status: 'completed' }),
    ]

    render(<ToolActivityFeed tools={tools} compact />)

    expect(screen.getByText('write_file')).toBeInTheDocument()
    expect(screen.getByText('read_file')).toBeInTheDocument()
  })

  it('shows completed icon for completed tools', () => {
    const tools: StreamToolUse[] = [
      makeTool({ toolName: 'search', toolId: 't1', status: 'completed' }),
    ]

    render(<ToolActivityFeed tools={tools} />)

    expect(screen.getByTestId('CheckCircleOutlinedIcon')).toBeInTheDocument()
  })
})
