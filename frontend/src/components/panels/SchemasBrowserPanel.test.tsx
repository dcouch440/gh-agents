import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { SchemasBrowserPanel } from './SchemasBrowserPanel'
import type { OutputSchema } from '@/types/schema'
import type { WorkflowStep } from '@/types/workflow'

const testSchema: OutputSchema = {
  id: 'schema-001',
  name: 'Test Schema',
  schema: { type: 'object', properties: { result: { type: 'string' } } },
  created_at: '2025-01-01T00:00:00Z',
}

const testSchema2: OutputSchema = {
  ...testSchema,
  id: 'schema-002',
  name: 'Review Output',
  schema: { properties: { approved: { type: 'boolean' }, comments: { type: 'string' }, score: { type: 'number' } } },
}

const testStep: WorkflowStep = {
  id: 'step-001',
  workflow_id: 'wf-001',
  agent_id: '',
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

const { mockFetchIfStale, mockUpdateStep, _schemas, _loading, _selectedStepIds, _steps } = vi.hoisted(() => ({
  mockFetchIfStale: vi.fn(),
  mockUpdateStep: vi.fn(),
  _schemas: { value: [] as OutputSchema[] },
  _loading: { value: false },
  _selectedStepIds: { value: new Set<string>() },
  _steps: { value: [] as WorkflowStep[] },
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
  outputSchemaStore: {
    store: 'schema',
    selectAll: () => _schemas.value,
    selectLoading: () => _loading.value,
    fetchIfStale: mockFetchIfStale,
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
    PORT_ARRAY: '#2dd4bf',
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
  _schemas.value = [testSchema, testSchema2]
  _loading.value = false
  _selectedStepIds.value = new Set()
  _steps.value = [testStep]
})

describe('SchemasBrowserPanel', () => {
  it('calls fetchIfStale on mount', () => {
    render(<SchemasBrowserPanel />)
    expect(mockFetchIfStale).toHaveBeenCalledOnce()
  })

  it('renders schema list with field counts', () => {
    render(<SchemasBrowserPanel />)
    expect(screen.getByText('Test Schema')).toBeInTheDocument()
    expect(screen.getByText('Review Output')).toBeInTheDocument()
    expect(screen.getByText('3 field(s)')).toBeInTheDocument()
  })

  it('shows empty state when no schemas', () => {
    _schemas.value = []
    render(<SchemasBrowserPanel />)
    expect(screen.getByText('No schemas found')).toBeInTheDocument()
  })

  it('filters schemas by search query', async () => {
    const user = userEvent.setup()
    render(<SchemasBrowserPanel />)

    await user.type(screen.getByPlaceholderText('Search schemas...'), 'Review')

    await vi.waitFor(() => {
      expect(screen.getByText('Review Output')).toBeInTheDocument()
      expect(screen.queryByText('Test Schema')).not.toBeInTheDocument()
    })
  })

  it('assigns schema to selected step on click', async () => {
    _selectedStepIds.value = new Set(['step-001'])
    const user = userEvent.setup()
    render(<SchemasBrowserPanel />)

    await user.click(screen.getByText('Review Output'))
    expect(mockUpdateStep).toHaveBeenCalledWith('step-001', { output_schema_id: 'schema-002' })
  })
})
