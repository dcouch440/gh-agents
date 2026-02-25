import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { agentTraceStore } from '@/stores/agentTraceStore'
import type { WsWireMessage } from '@/types/ws'
import { AgentTracePanel } from './AgentTracePanel'

const wire = (event: string, data: Record<string, unknown>, ts = '2025-01-01T00:00:00Z'): WsWireMessage => ({
  topic: 'workflow',
  event,
  ts,
  run_id: 'run-1',
  user_id: 'user-1',
  seq: 1,
  data,
})

describe('AgentTracePanel', () => {
  beforeEach(() => {
    agentTraceStore.reset()
  })

  it('shows empty state when no traces', () => {
    render(<AgentTracePanel />)
    expect(screen.getByText('No agent traces yet')).toBeInTheDocument()
  })

  it('renders agent name from trace', () => {
    agentTraceStore.handleWsEvent(wire('debug_system_prompt', {
      workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'Researcher', content: 'You research.',
    }))

    render(<AgentTracePanel />)
    expect(screen.getByText('Researcher')).toBeInTheDocument()
  })

  it('renders fallback name when agent_name is null', () => {
    agentTraceStore.handleWsEvent(wire('debug_system_prompt', {
      workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: null, content: 'prompt',
    }))

    render(<AgentTracePanel />)
    expect(screen.getByText('Agent')).toBeInTheDocument()
  })

  it('renders multiple agents in order', () => {
    agentTraceStore.handleWsEvent(wire('debug_system_prompt', {
      workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'Alpha', content: 'p1',
    }))
    agentTraceStore.handleWsEvent(wire('debug_system_prompt', {
      workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-2', agent_name: 'Beta', content: 'p2',
    }))

    render(<AgentTracePanel />)
    expect(screen.getByText('Alpha')).toBeInTheDocument()
    expect(screen.getByText('Beta')).toBeInTheDocument()
  })

  it('expands to show system prompt when clicked', () => {
    agentTraceStore.handleWsEvent(wire('debug_system_prompt', {
      workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'Bot', content: 'You are a helpful bot.',
    }))

    render(<AgentTracePanel />)

    // Initially collapsed - system prompt not visible
    expect(screen.queryByText('You are a helpful bot.')).not.toBeInTheDocument()

    // Click agent header to expand
    fireEvent.click(screen.getByText('Bot'))

    // Now the System Prompt section label is visible
    expect(screen.getByText('System Prompt')).toBeInTheDocument()
  })
})
