import { describe, it, expect } from 'vitest'
import { toRFNodes, toRFEdges } from './mappers'
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

describe('toRFNodes', () => {
  it('maps WorkflowStep array to React Flow nodes', () => {
    const nodes = toRFNodes([step1, step2], new Set())

    expect(nodes).toHaveLength(2)
    expect(nodes[0]).toEqual({
      id: 'step-001',
      type: 'stepNode',
      position: { x: 100, y: 200 },
      selected: false,
      data: {
        label: 'First Step',
        stepType: 'single',
        agentId: 'agent-001',
        promptTemplateId: null,
        outputSchemaId: null,
      },
    })
  })

  it('marks selected nodes', () => {
    const nodes = toRFNodes([step1, step2], new Set(['step-002']))

    expect(nodes[0]?.selected).toBe(false)
    expect(nodes[1]?.selected).toBe(true)
  })

  it('returns empty array for empty input', () => {
    expect(toRFNodes([], new Set())).toEqual([])
  })

  it('falls back to execution_mode when name is null', () => {
    const stepNoName: WorkflowStep = { ...step1, name: null }
    const nodes = toRFNodes([stepNoName], new Set())
    expect(nodes[0]?.data.label).toBe('single')
  })
})

describe('toRFEdges', () => {
  it('maps WorkflowStepEdge array to React Flow edges', () => {
    const edges = toRFEdges([edge1], new Set())

    expect(edges).toHaveLength(1)
    expect(edges[0]).toEqual({
      id: 'edge-001',
      type: 'stepEdge',
      source: 'step-001',
      target: 'step-002',
      selected: false,
    })
  })

  it('marks selected edges', () => {
    const edges = toRFEdges([edge1], new Set(['edge-001']))
    expect(edges[0]?.selected).toBe(true)
  })

  it('returns empty array for empty input', () => {
    expect(toRFEdges([], new Set())).toEqual([])
  })
})
