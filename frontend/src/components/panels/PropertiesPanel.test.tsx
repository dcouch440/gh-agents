import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import { PropertiesPanel } from './PropertiesPanel'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'

const step1: WorkflowStep = {
  id: 'step-001',
  workflow_id: 'wf-001',
  name: 'First Step',
  agent_id: 'agent-001',
  execution_mode: 'single',
  for_each_ref: null,
  prompt_template_id: null,
  prompt_template: '{task_input}',
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
  system_prompt_suffix: null,
}

const step2: WorkflowStep = { ...step1, id: 'step-002', name: 'Second Step' }

const edge1: WorkflowStepEdge = {
  id: 'edge-001',
  from_step_id: 'step-001',
  to_step_id: 'step-002',
}

const { _selectedStepIds, _selectedEdgeIds, _steps, _edges } = vi.hoisted(() => ({
  _selectedStepIds: { value: new Set<string>() },
  _selectedEdgeIds: { value: new Set<string>() },
  _steps: { value: [] as WorkflowStep[] },
  _edges: { value: [] as WorkflowStepEdge[] },
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
  canvasStore: {
    store: 'canvas',
    selectSelectedStepIds: () => _selectedStepIds.value,
    selectSelectedEdgeIds: () => _selectedEdgeIds.value,
  },
  workflowStore: {
    store: 'workflow',
    selectSteps: () => _steps.value,
    selectEdges: () => _edges.value,
    selectStepById: (id: string | null) => () => (id ? (_steps.value.find((s: WorkflowStep) => s.id === id) ?? null) : null),
    selectEdgeById: (id: string | null) => () => (id ? (_edges.value.find((e: WorkflowStepEdge) => e.id === id) ?? null) : null),
    updateStep: vi.fn(),
  },
  agentStore: {
    store: 'agent',
    selectAll: () => [],
    selectLoading: () => false,
    selectById: () => () => undefined,
    fetchAll: vi.fn(),
  },
  promptTemplateStore: {
    store: 'prompt',
    selectAll: () => [],
    selectLoading: () => false,
    selectById: () => () => undefined,
    fetchIfStale: vi.fn(),
  },
  outputSchemaStore: {
    store: 'schema',
    selectAll: () => [],
    selectLoading: () => false,
    selectById: () => () => undefined,
    fetchIfStale: vi.fn(),
  },
  layoutStore: {
    openRightPanel: vi.fn(),
  },
  protocolStore: {
    store: 'protocol',
    selectAll: () => [],
    fetchAll: vi.fn(),
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
  _selectedStepIds.value = new Set()
  _selectedEdgeIds.value = new Set()
  _steps.value = [step1, step2]
  _edges.value = [edge1]
})

describe('PropertiesPanel', () => {
  it('shows empty state when nothing selected', () => {
    render(<PropertiesPanel />)
    expect(screen.getByText('Select a node to view properties')).toBeInTheDocument()
  })

  it('renders step properties when step selected', () => {
    _selectedStepIds.value = new Set(['step-001'])
    render(<PropertiesPanel />)
    expect(screen.getByDisplayValue('First Step')).toBeInTheDocument()
    expect(screen.getByText('single')).toBeInTheDocument()
  })

  it('renders edge properties when edge selected', () => {
    _selectedEdgeIds.value = new Set(['edge-001'])
    render(<PropertiesPanel />)
    expect(screen.getByText('Connection')).toBeInTheDocument()
  })

  it('prioritizes step over edge when both selected', () => {
    _selectedStepIds.value = new Set(['step-001'])
    _selectedEdgeIds.value = new Set(['edge-001'])
    render(<PropertiesPanel />)
    expect(screen.getByDisplayValue('First Step')).toBeInTheDocument()
    expect(screen.queryByText('Connection')).not.toBeInTheDocument()
  })
})
