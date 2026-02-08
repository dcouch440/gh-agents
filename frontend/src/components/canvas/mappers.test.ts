import { describe, it, expect } from 'vitest'
import { toRFNodes, toRFEdges } from './mappers'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'

const step1: WorkflowStep = {
  id: 'step-001',
  workflow_id: 'wf-001',
  name: 'First Step',
  description: 'The first step',
  step_type: 'llm',
  agent_id: 'agent-001',
  prompt_template_id: null,
  output_schema_id: null,
  for_each_label_field: null,
  config: null,
  position_x: 100,
  position_y: 200,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

const step2: WorkflowStep = {
  ...step1,
  id: 'step-002',
  name: 'Second Step',
  description: null,
  step_type: 'for_each',
  agent_id: null,
  position_x: 400,
  position_y: 100,
}

const edge1: WorkflowStepEdge = {
  id: 'edge-001',
  workflow_id: 'wf-001',
  from_step_id: 'step-001',
  to_step_id: 'step-002',
  condition: 'always',
  created_at: '2025-01-01T00:00:00Z',
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
        stepType: 'llm',
        description: 'The first step',
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

  it('preserves null description', () => {
    const nodes = toRFNodes([step2], new Set())
    expect(nodes[0]?.data.description).toBeNull()
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
      data: { condition: 'always' },
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
