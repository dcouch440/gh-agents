import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import { agentTraceStore } from '@/stores/agentTraceStore'
import { activityStore } from '@/stores/activity'
import { workflowStore } from '@/stores/workflowStore'
import { createNormalizedMap, nmFromArray } from '@/stores/lib'
import { RunTab } from './RunTab'
import type { WorkflowStep } from '@/types/workflow'

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
})

describe('RunTab', () => {
  beforeEach(() => {
    agentTraceStore.reset()
    activityStore.store.setState({ entries: [], maxSize: 500 })
    workflowStore.store.setState({
      steps: createNormalizedMap(),
      edges: createNormalizedMap(),
    })
  })

  it('shows empty state when no traces', () => {
    render(<RunTab />)
    expect(screen.getByText(/no execution traces yet/i)).toBeInTheDocument()
  })

  it('renders agent traces grouped by step', () => {
    agentTraceStore.store.setState({
      traces: {
        'exec-a': {
          agentExecutionId: 'exec-a',
          agentName: 'Researcher',
          stepId: 'step-1',
          events: [],
        },
        'exec-b': {
          agentExecutionId: 'exec-b',
          agentName: 'Writer',
          stepId: 'step-2',
          events: [],
        },
      },
      order: ['exec-a', 'exec-b'],
    })

    workflowStore.store.setState({
      steps: nmFromArray([
        makeStep('step-1', 'Research Step'),
        makeStep('step-2', 'Write Step'),
      ]),
    })

    render(<RunTab />)

    expect(screen.getByText('Research Step')).toBeInTheDocument()
    expect(screen.getByText('Write Step')).toBeInTheDocument()
    expect(screen.getByText('Researcher')).toBeInTheDocument()
    expect(screen.getByText('Writer')).toBeInTheDocument()
  })

  it('renders activity timeline when activities exist', () => {
    activityStore.store.setState({
      entries: [
        {
          id: 'act_1',
          seq: 1,
          event: { type: 'workflow:started', workflowId: 'wf-1', totalSteps: 3 },
          ts: new Date().toISOString(),
          runId: 'run-1',
          userId: null,
          receivedAt: Date.now(),
        },
      ],
      maxSize: 500,
    })

    render(<RunTab />)

    expect(screen.getByText('Activity')).toBeInTheDocument()
    expect(screen.getByText('1 event(s)')).toBeInTheDocument()
  })
})
