import { describe, it, expect } from 'vitest'
import { toRFNodes, toRFEdges } from './mappers'
import type { StepNodeLookups } from './mappers'
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
  position_x: 100,
  position_y: 200,
  system_prompt_suffix: null,
}

const step2: WorkflowStep = {
  ...step1,
  id: 'step-002',
  name: 'Second Step',
  execution_mode: 'for_each',
  agent_id: 'agent-001',
  position_x: 400,
  position_y: 100,
}

const edge1: WorkflowStepEdge = {
  id: 'edge-001',
  from_step_id: 'step-001',
  to_step_id: 'step-002',
}

const emptyLookups: StepNodeLookups = {
  agents: new Map(),
  outputSchemas: new Map(),
  stepNames: new Map(),
  edges: [],
}

describe('toRFNodes', () => {
  it('maps WorkflowStep array to React Flow nodes', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      stepNames: new Map([['step-001', 'First Step'], ['step-002', 'Second Step']]),
    }
    const nodes = toRFNodes([step1, step2], lookups)

    expect(nodes).toHaveLength(2)
    expect(nodes[0]).toEqual({
      id: 'step-001',
      type: 'stepNode',
      position: { x: 100, y: 200 },
      data: {
        label: 'First Step',
        stepType: 'single',
        agentId: 'agent-001',
        promptTemplateId: null,
        outputSchemaId: null,
        agentName: null,
        modelId: null,
        outputSchemaName: null,
        upstreamStepNames: [],
      },
    })
  })

  it('returns empty array for empty input', () => {
    expect(toRFNodes([], emptyLookups)).toEqual([])
  })

  it('falls back to execution_mode when name is null', () => {
    const stepNoName: WorkflowStep = { ...step1, name: null }
    const nodes = toRFNodes([stepNoName], emptyLookups)
    expect(nodes[0]?.data.label).toBe('single')
  })

  it('resolves agent name and model from lookups', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      agents: new Map([['agent-001', { name: 'TestBot', model_id: 'claude-sonnet-4' }]]),
    }
    const nodes = toRFNodes([step1], lookups)
    expect(nodes[0]?.data.agentName).toBe('TestBot')
    expect(nodes[0]?.data.modelId).toBe('claude-sonnet-4')
  })

  it('resolves output schema name from lookups', () => {
    const stepWithSchema: WorkflowStep = { ...step1, output_schema_id: 'schema-001' }
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      outputSchemas: new Map([['schema-001', { name: 'ReviewSchema' }]]),
    }
    const nodes = toRFNodes([stepWithSchema], lookups)
    expect(nodes[0]?.data.outputSchemaName).toBe('ReviewSchema')
  })

  it('computes upstream step names from edges', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      stepNames: new Map([['step-001', 'First Step'], ['step-002', 'Second Step']]),
      edges: [{ from_step_id: 'step-001', to_step_id: 'step-002' }],
    }
    const nodes = toRFNodes([step1, step2], lookups)
    expect(nodes[1]?.data.upstreamStepNames).toEqual(['First Step'])
    expect(nodes[0]?.data.upstreamStepNames).toEqual([])
  })

  it('falls back to "Unknown Step" when upstream step not in stepNames', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      edges: [{ from_step_id: 'step-999', to_step_id: 'step-001' }],
    }
    const nodes = toRFNodes([step1], lookups)
    expect(nodes[0]?.data.upstreamStepNames).toEqual(['Unknown Step'])
  })
})

describe('toRFEdges', () => {
  it('maps WorkflowStepEdge array to React Flow edges', () => {
    const edges = toRFEdges([edge1])

    expect(edges).toHaveLength(1)
    expect(edges[0]).toEqual({
      id: 'edge-001',
      type: 'stepEdge',
      source: 'step-001',
      target: 'step-002',
    })
  })

  it('returns empty array for empty input', () => {
    expect(toRFEdges([])).toEqual([])
  })
})
