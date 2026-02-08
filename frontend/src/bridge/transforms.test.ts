import { stepToNode, edgeToFlowEdge, nodeToPositionUpdate } from './transforms'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import type { StepExecutionState } from '@/stores/workflowExecutionStore'

// ── Fixtures ────────────────────────────────────────────────────────────────

const makeStep = (overrides: Partial<WorkflowStep> = {}): WorkflowStep => ({
  id: 's1',
  workflow_id: 'w1',
  agent_id: 'a1',
  execution_mode: 'single',
  for_each_ref: null,
  prompt_template_id: 'pt1',
  prompt_template: '',
  output_schema_id: null,
  output_variable_name: null,
  interactive_agent_id: null,
  for_each_label_field: null,
  display_order: 0,
  version: 1,
  reasoning_trace: false,
  verification_agent_ids: [],
  position_x: 100,
  position_y: 200,
  name: 'Step One',
  ...overrides,
})

const makeEdge = (overrides: Partial<WorkflowStepEdge> = {}): WorkflowStepEdge => ({
  id: 'e1',
  from_step_id: 's1',
  to_step_id: 's2',
  ...overrides,
})

const makeExecState = (overrides: Partial<StepExecutionState> = {}): StepExecutionState => ({
  status: 'running',
  output: null,
  error: null,
  inputTokens: null,
  outputTokens: null,
  durationMs: null,
  forEachProgress: null,
  startedAt: null,
  completedAt: null,
  ...overrides,
})

// ── stepToNode ──────────────────────────────────────────────────────────────

describe('stepToNode', () => {
  it('transforms a single step to a singleStep node', () => {
    const node = stepToNode(makeStep(), null, false, false)

    expect(node.id).toBe('s1')
    expect(node.type).toBe('singleStep')
    expect(node.position).toEqual({ x: 100, y: 200 })
    expect(node.selected).toBe(false)
    expect(node.data.name).toBe('Step One')
    expect(node.data.stepType).toBe('single')
    expect(node.data.agentId).toBe('a1')
    expect(node.data.executionState).toBeNull()
    expect(node.data.hovered).toBe(false)
  })

  it('transforms a for_each step to a forEachStep node', () => {
    const step = makeStep({
      execution_mode: 'for_each',
      for_each_label_field: 'category',
    })
    const node = stepToNode(step, null, false, false)

    expect(node.type).toBe('forEachStep')
    expect(node.data.stepType).toBe('for_each')
    expect(node.data.forEachLabelField).toBe('category')
  })

  it('transforms a room step to a roomStep node', () => {
    const step = makeStep({ execution_mode: 'room' })
    const node = stepToNode(step, null, false, false)

    expect(node.type).toBe('roomStep')
    expect(node.data.stepType).toBe('room')
  })

  it('falls back to singleStep for unknown step types', () => {
    const step = makeStep({ execution_mode: 'unknown_type' })
    const node = stepToNode(step, null, false, false)

    expect(node.type).toBe('singleStep')
    expect(node.data.stepType).toBe('unknown_type')
  })

  it('sets selected on the node object', () => {
    const node = stepToNode(makeStep(), null, true, false)

    expect(node.selected).toBe(true)
  })

  it('sets hovered in node data', () => {
    const node = stepToNode(makeStep(), null, false, true)

    expect(node.data.hovered).toBe(true)
  })

  it('passes through execution state', () => {
    const exec = makeExecState({ status: 'success', output: 'done' })
    const node = stepToNode(makeStep(), exec, false, false)

    expect(node.data.executionState).toBe(exec)
    expect(node.data.executionState?.status).toBe('success')
    expect(node.data.executionState?.output).toBe('done')
  })

  it('maps position_x/y to position.x/y', () => {
    const step = makeStep({ position_x: 350, position_y: 475 })
    const node = stepToNode(step, null, false, false)

    expect(node.position).toEqual({ x: 350, y: 475 })
  })
})

// ── edgeToFlowEdge ──────────────────────────────────────────────────────────

describe('edgeToFlowEdge', () => {
  it('transforms edge to dataFlow type', () => {
    const edge = edgeToFlowEdge(makeEdge(), false, false)

    expect(edge.id).toBe('e1')
    expect(edge.source).toBe('s1')
    expect(edge.target).toBe('s2')
    expect(edge.type).toBe('dataFlow')
  })

  it('sets selected on the edge object', () => {
    const edge = edgeToFlowEdge(makeEdge(), true, false)

    expect(edge.selected).toBe(true)
  })

  it('sets hovered in edge data', () => {
    const edge = edgeToFlowEdge(makeEdge(), false, true)

    expect(edge.data?.hovered).toBe(true)
  })

  it('maps from_step_id/to_step_id to source/target', () => {
    const e = makeEdge({ from_step_id: 'src', to_step_id: 'dst' })
    const edge = edgeToFlowEdge(e, false, false)

    expect(edge.source).toBe('src')
    expect(edge.target).toBe('dst')
  })
})

// ── nodeToPositionUpdate ────────────────────────────────────────────────────

describe('nodeToPositionUpdate', () => {
  it('extracts position_x/y from node position', () => {
    const node = stepToNode(makeStep({ position_x: 500, position_y: 600 }), null, false, false)
    const update = nodeToPositionUpdate(node)

    expect(update).toEqual({ position_x: 500, position_y: 600 })
  })
})
