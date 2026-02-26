import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import { boardStore } from '@/stores/boardStore'
import { workflowStore } from '@/stores/workflowStore'
import { createNormalizedMap, nmFromArray } from '@/stores/lib'
import { DispatchTab } from './DispatchTab'
import type { BoardSubmitResponse } from '@/types/board'
import type { WorkflowStep } from '@/types/workflow'

vi.mock('../hooks/useDispatchPollAll', () => ({
  useDispatchPollAll: vi.fn(),
}))

const makeStep = (id: string, name: string): WorkflowStep => ({
  id,
  workflow_id: 'wf-1',
  agent_id: 'agent-1',
  execution_mode: 'workforce',
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
  position_x: null,
  position_y: null,
  width: null,
  height: null,
  name,
  room_id: null,
  system_prompt_suffix: null,
  description: '',
  sub_workflow_template_id: null,
  pinned: false,
  run_results_summary: '',
})

const makeBoardResponse = (dispatches: BoardSubmitResponse['dispatches'] = []): BoardSubmitResponse => ({
  is_first_submit: false,
  changeset: {
    agentless: { deleted_node_ids: [], deleted_edge_ids: [], rewired_edges: [], moved_nodes: [] },
    noise: [],
    meaningful: [],
    aggregate_score: 0,
    should_dispatch: dispatches.length > 0,
  },
  snapshot: { nodes: [], edges: [], global_notes: [] },
  phase_zero: {
    created_steps: [],
    created_edges: [],
    deleted_steps: [],
    deleted_edges: [],
    rewired_edges: [],
    moved_steps: [],
    updated_steps: [],
  },
  dispatches,
})

describe('DispatchTab', () => {
  beforeEach(() => {
    boardStore.store.setState({
      status: 'idle',
      error: null,
      lastResponse: null,
      isFirstSubmit: true,
      elementStepMap: {},
      elementEdgeMap: {},
    })
    workflowStore.store.setState({
      steps: createNormalizedMap(),
      edges: createNormalizedMap(),
    })
  })

  it('shows empty state when no dispatches', () => {
    render(<DispatchTab />)
    expect(screen.getByText(/no dispatches yet/i)).toBeInTheDocument()
  })

  it('renders dispatch rows when dispatches exist', () => {
    boardStore.store.setState({
      status: 'success',
      error: null,
      lastResponse: makeBoardResponse([
        { execution_id: 'exec-1', session_id: 'sess-1', step_id: 'step-1', instruction: 'Configure Research node' },
        { execution_id: 'exec-2', session_id: 'sess-2', step_id: 'step-2', instruction: 'Configure Writer node' },
      ]),
      isFirstSubmit: false,
      elementStepMap: {},
      elementEdgeMap: {},
    })

    workflowStore.store.setState({
      steps: nmFromArray([
        makeStep('step-1', 'Research'),
        makeStep('step-2', 'Writer'),
      ]),
    })

    render(<DispatchTab />)

    expect(screen.getByText('Research')).toBeInTheDocument()
    expect(screen.getByText('Writer')).toBeInTheDocument()
  })

  it('falls back to truncated step ID when step name is missing', () => {
    boardStore.store.setState({
      status: 'success',
      error: null,
      lastResponse: makeBoardResponse([
        { execution_id: 'exec-1', session_id: 'sess-1', step_id: 'abcdef12-3456-7890', instruction: 'Configure' },
      ]),
      isFirstSubmit: false,
      elementStepMap: {},
      elementEdgeMap: {},
    })

    render(<DispatchTab />)

    expect(screen.getByText('abcdef12')).toBeInTheDocument()
  })
})
