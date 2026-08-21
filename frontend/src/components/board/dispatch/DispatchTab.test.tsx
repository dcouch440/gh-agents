import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import { boardStore } from '@/stores/boardStore'
import { workflowStore } from '@/stores/workflowStore'
import { workflowLiveStore } from '@/stores/workflowLiveStore'
import { createNormalizedMap, nmFromArray } from '@/stores/lib'
import { DispatchTab } from './DispatchTab'
import type { LiveDispatch } from '@/stores/workflowLiveStore'
import type { WorkflowStep } from '@/types/workflow'

const makeDispatch = (stepId: string, executionId: string, instruction: string): LiveDispatch => ({
  stepId,
  executionId,
  status: 'completed',
  instruction,
  createdAt: '2025-01-01T00:00:00Z',
  result: null,
  traceLen: 0,
  source: 'registry',
})

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

  pinned: false,
  run_results_summary: '',
  designer_handoff: '',
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
    workflowLiveStore.store.setState({
      workflowId: 'wf-1',
      baselineByStep: {},
      dispatches: [],
      runSteps: [],
      isGenerating: false,
      loading: false,
      error: null,
      hydratedAt: null,
    })
  })

  it('shows empty state when no dispatches', () => {
    render(<DispatchTab />)
    expect(screen.getByText(/no dispatches yet/i)).toBeInTheDocument()
  })

  it('renders a row per dispatch reported by the server', () => {
    workflowLiveStore.store.setState({
      dispatches: [
        makeDispatch('step-1', 'exec-1', 'Configure Research node'),
        makeDispatch('step-2', 'exec-2', 'Configure Writer node'),
      ],
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

  it('renders after a refresh, when there is no board-submit response at all', () => {
    // The refresh case: `lastResponse` is only written by board submit, never by
    // Generate, so rows must not depend on it.
    boardStore.store.setState({ lastResponse: null })
    workflowLiveStore.store.setState({
      dispatches: [makeDispatch('step-1', 'exec-1', 'Configure Research node')],
    })
    workflowStore.store.setState({ steps: nmFromArray([makeStep('step-1', 'Research')]) })

    render(<DispatchTab />)

    expect(screen.getByText('Research')).toBeInTheDocument()
    expect(screen.queryByText(/no dispatches yet/i)).not.toBeInTheDocument()
  })

  it('preserves the server ordering, which is newest-first per step', () => {
    // Regression guard: the old hook read tasks[length - 1] against a
    // newest-first list and hydrated the oldest dispatch.
    workflowLiveStore.store.setState({
      dispatches: [
        makeDispatch('step-2', 'exec-newest', 'Newest'),
        makeDispatch('step-1', 'exec-older', 'Older'),
      ],
    })
    workflowStore.store.setState({
      steps: nmFromArray([makeStep('step-1', 'Older step'), makeStep('step-2', 'Newest step')]),
    })

    render(<DispatchTab />)

    const rows = screen.getAllByText(/step$/)
    expect(rows[0]).toHaveTextContent('Newest step')
    expect(rows[1]).toHaveTextContent('Older step')
  })

  it('falls back to truncated step ID when step name is missing', () => {
    workflowLiveStore.store.setState({
      dispatches: [makeDispatch('abcdef12-3456-7890', 'exec-1', 'Configure')],
    })

    render(<DispatchTab />)

    expect(screen.getByText('abcdef12')).toBeInTheDocument()
  })
})
