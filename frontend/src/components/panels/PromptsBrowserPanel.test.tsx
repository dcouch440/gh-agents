import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PromptsBrowserPanel } from './PromptsBrowserPanel'
import type { PromptTemplate } from '@/types/template'
import type { WorkflowStep } from '@/types/workflow'

const testTemplate: PromptTemplate = {
  id: 'template-001',
  user_id: 'user-001',
  name: 'Test Template',
  description: 'A test prompt template',
  template: 'Hello {{name}}, please {{action}}',
  variables: ['name', 'action'],
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

const testTemplate2: PromptTemplate = {
  ...testTemplate,
  id: 'template-002',
  name: 'Code Review',
  variables: ['code', 'language', 'context'],
}

const testStep: WorkflowStep = {
  id: 'step-001',
  workflow_id: 'wf-001',
  agent_id: '',
  execution_mode: 'llm',
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
}

const {
  mockFetchIfStale,
  mockUpdateStep,
  _templates,
  _loading,
  _selectedStepIds,
  _steps,
} = vi.hoisted(() => ({
  mockFetchIfStale: vi.fn(),
  mockUpdateStep: vi.fn(),
  _templates: { value: [] as PromptTemplate[] },
  _loading: { value: false },
  _selectedStepIds: { value: new Set<string>() },
  _steps: { value: [] as WorkflowStep[] },
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
  promptTemplateStore: {
    store: 'prompt',
    selectAll: () => _templates.value,
    selectLoading: () => _loading.value,
    fetchIfStale: mockFetchIfStale,
  },
  canvasStore: {
    store: 'canvas',
    selectSelectedStepIds: () => _selectedStepIds.value,
  },
  workflowStore: {
    store: 'workflow',
    selectSteps: () => _steps.value,
    updateStep: mockUpdateStep,
  },
}))

vi.mock('@/constants', () => ({
  DESIGN: {
    PORT_JSON: '#a78bfa',
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
  _templates.value = [testTemplate, testTemplate2]
  _loading.value = false
  _selectedStepIds.value = new Set()
  _steps.value = [testStep]
})

describe('PromptsBrowserPanel', () => {
  it('calls fetchIfStale on mount', () => {
    render(<PromptsBrowserPanel />)
    expect(mockFetchIfStale).toHaveBeenCalledOnce()
  })

  it('renders template list with variable counts', () => {
    render(<PromptsBrowserPanel />)
    expect(screen.getByText('Test Template')).toBeInTheDocument()
    expect(screen.getByText('2 variable(s)')).toBeInTheDocument()
    expect(screen.getByText('Code Review')).toBeInTheDocument()
    expect(screen.getByText('3 variable(s)')).toBeInTheDocument()
  })

  it('shows empty state when no templates', () => {
    _templates.value = []
    render(<PromptsBrowserPanel />)
    expect(screen.getByText('No templates found')).toBeInTheDocument()
  })

  it('filters templates by search query', async () => {
    const user = userEvent.setup()
    render(<PromptsBrowserPanel />)

    await user.type(screen.getByPlaceholderText('Search templates...'), 'Review')

    await vi.waitFor(() => {
      expect(screen.getByText('Code Review')).toBeInTheDocument()
      expect(screen.queryByText('Test Template')).not.toBeInTheDocument()
    })
  })

  it('assigns template to selected step on click', async () => {
    _selectedStepIds.value = new Set(['step-001'])
    const user = userEvent.setup()
    render(<PromptsBrowserPanel />)

    await user.click(screen.getByText('Code Review'))
    expect(mockUpdateStep).toHaveBeenCalledWith('step-001', { prompt_template_id: 'template-002' })
  })
})
