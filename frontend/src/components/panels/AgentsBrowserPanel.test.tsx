import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { AgentsBrowserPanel } from './AgentsBrowserPanel'
import type { Agent } from '@/types/agent'
import type { WorkflowStep } from '@/types/workflow'

const testAgent: Agent = {
  id: 'agent-001',
  name: 'TestBot',
  system_prompt: 'You are a test agent.',
  model_provider: 'anthropic',
  model_id: 'claude-sonnet-4-20250514',
  model_max_tokens: 8192,
  model_temperature: 0.7,
  status: 'idle',
  output_schema_id: null,
  version: 1,
}

const testAgent2: Agent = {
  ...testAgent,
  id: 'agent-002',
  name: 'CodeBot',
  model_id: 'claude-opus-4-20250514',
}

const testStep: WorkflowStep = {
  id: 'step-001',
  workflow_id: 'wf-001',
  agent_id: 'agent-001',
  execution_mode: 'single',
  for_each_ref: null,
  prompt_template_id: null,
  prompt_template: '',
  output_schema_id: null,
  output_variable_name: null,
  interactive_agent_id: null,
  for_each_label_field: null,
  display_order: 0,
  version: 1,
  reasoning_trace: false,
  verification_agent_ids: [],
  position_x: 0,
  position_y: 0,
  name: 'First Step',
  system_prompt_suffix: null,
}

const { mockFetchAll, mockUpdateStep, _agents, _loading, _selectedStepIds, _steps } = vi.hoisted(() => ({
  mockFetchAll: vi.fn(),
  mockUpdateStep: vi.fn(),
  _agents: { value: [] as Agent[] },
  _loading: { value: false },
  _selectedStepIds: { value: new Set<string>() },
  _steps: { value: [] as WorkflowStep[] },
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
  agentStore: {
    store: 'agent',
    selectAll: () => _agents.value,
    selectLoading: () => _loading.value,
    fetchAll: mockFetchAll,
  },
  canvasStore: {
    store: 'canvas',
    selectSelectedStepIds: () => _selectedStepIds.value,
  },
  workflowStore: {
    store: 'workflow',
    selectStepById: (id: string | null) => () => (id ? (_steps.value.find((s: WorkflowStep) => s.id === id) ?? null) : null),
    updateStep: mockUpdateStep,
  },
}))

vi.mock('@/constants', () => ({
  DESIGN: {
    PORT_STRING: '#3b82f6',
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
  _agents.value = [testAgent, testAgent2]
  _loading.value = false
  _selectedStepIds.value = new Set()
  _steps.value = [testStep]
})

describe('AgentsBrowserPanel', () => {
  it('calls fetchAll on mount', () => {
    render(<AgentsBrowserPanel />)
    expect(mockFetchAll).toHaveBeenCalledOnce()
  })

  it('renders agent list', () => {
    render(<AgentsBrowserPanel />)
    expect(screen.getByText('TestBot')).toBeInTheDocument()
    expect(screen.getByText('CodeBot')).toBeInTheDocument()
  })

  it('shows empty state when no agents', () => {
    _agents.value = []
    render(<AgentsBrowserPanel />)
    expect(screen.getByText('No agents found')).toBeInTheDocument()
  })

  it('filters agents by search query', async () => {
    const user = userEvent.setup()
    render(<AgentsBrowserPanel />)

    const input = screen.getByPlaceholderText('Search agents...')
    await user.type(input, 'Code')

    await vi.waitFor(() => {
      expect(screen.getByText('CodeBot')).toBeInTheDocument()
      expect(screen.queryByText('TestBot')).not.toBeInTheDocument()
    })
  })

  it('shows contextual empty state for search with no results', async () => {
    const user = userEvent.setup()
    render(<AgentsBrowserPanel />)

    const input = screen.getByPlaceholderText('Search agents...')
    await user.type(input, 'zzz')

    await vi.waitFor(() => {
      expect(screen.getByText('No agents matching "zzz"')).toBeInTheDocument()
    })
  })

  it('assigns agent to selected step on click', async () => {
    _selectedStepIds.value = new Set(['step-001'])
    const user = userEvent.setup()
    render(<AgentsBrowserPanel />)

    await user.click(screen.getByText('CodeBot'))
    expect(mockUpdateStep).toHaveBeenCalledWith('step-001', { agent_id: 'agent-002' })
  })

  it('does not assign when no step selected', async () => {
    const user = userEvent.setup()
    render(<AgentsBrowserPanel />)

    await user.click(screen.getByText('CodeBot'))
    expect(mockUpdateStep).not.toHaveBeenCalled()
  })

  it('shows loading spinner when loading', () => {
    _loading.value = true
    render(<AgentsBrowserPanel />)
    expect(screen.getByText('Loading agents...')).toBeInTheDocument()
  })
})
