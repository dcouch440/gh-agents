import { describe, it, expect } from 'vitest'
import { buildVariableCompletions } from './variableContext'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import type { OutputSchema } from '@/types/schema'

const baseStep: WorkflowStep = {
  id: 'step-001',
  workflow_id: 'wf-001',
  name: 'Parse Input',
  agent_id: 'agent-001',
  execution_mode: 'single',
  for_each_ref: null,
  prompt_template_id: null,
  prompt_template: '{task_input}',
  output_schema_id: 'schema-001',
  output_variable_name: 'parse_output',
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

const currentStep: WorkflowStep = {
  ...baseStep,
  id: 'step-002',
  name: 'Process',
  output_schema_id: null,
  output_variable_name: null,
}

const schema: OutputSchema = {
  id: 'schema-001',
  name: 'ParseSchema',
  schema: {
    type: 'object',
    properties: {
      summary: { type: 'string', description: 'Brief summary' },
      count: { type: 'number' },
    },
  },
  created_at: '2025-01-01T00:00:00Z',
}

const edge: WorkflowStepEdge = {
  id: 'edge-001',
  from_step_id: 'step-001',
  to_step_id: 'step-002',
}

describe('buildVariableCompletions', () => {
  it('builds completions from upstream step with output schema', () => {
    const schemas = new Map([['schema-001', schema]])
    const completions = buildVariableCompletions(
      'step-002',
      new Map([['step-001', baseStep], ['step-002', currentStep]]),
      [edge],
      schemas,
    )

    expect(completions).toEqual([
      {
        label: '{parse_output}',
        displayLabel: 'parse_output',
        detail: 'object \u2014 from Parse Input',
        section: 'Parse Input',
      },
      {
        label: '{parse_output.summary}',
        displayLabel: 'parse_output.summary',
        detail: 'string \u2014 from Parse Input',
        section: 'Parse Input',
      },
      {
        label: '{parse_output.count}',
        displayLabel: 'parse_output.count',
        detail: 'number \u2014 from Parse Input',
        section: 'Parse Input',
      },
    ])
  })

  it('skips steps without output_variable_name', () => {
    const stepNoVar: WorkflowStep = { ...baseStep, output_variable_name: null }
    const schemas = new Map([['schema-001', schema]])
    const completions = buildVariableCompletions(
      'step-002',
      new Map([['step-001', stepNoVar], ['step-002', currentStep]]),
      [edge],
      schemas,
    )

    expect(completions).toEqual([])
  })

  it('skips steps without output_schema_id', () => {
    const stepNoSchema: WorkflowStep = { ...baseStep, output_schema_id: null }
    const schemas = new Map([['schema-001', schema]])
    const completions = buildVariableCompletions(
      'step-002',
      new Map([['step-001', stepNoSchema], ['step-002', currentStep]]),
      [edge],
      schemas,
    )

    expect(completions).toEqual([])
  })

  it('skips steps whose schema is not in the map', () => {
    const emptySchemas: ReadonlyMap<string, OutputSchema> = new Map()
    const completions = buildVariableCompletions(
      'step-002',
      new Map([['step-001', baseStep], ['step-002', currentStep]]),
      [edge],
      emptySchemas,
    )

    expect(completions).toEqual([])
  })

  it('returns empty array when no upstream steps', () => {
    const schemas = new Map([['schema-001', schema]])
    const completions = buildVariableCompletions(
      'step-002',
      new Map([['step-001', baseStep], ['step-002', currentStep]]),
      [],
      schemas,
    )

    expect(completions).toEqual([])
  })

  it('groups completions by step name', () => {
    const step3: WorkflowStep = {
      ...baseStep,
      id: 'step-003',
      name: 'Fetch Data',
      output_variable_name: 'fetch_output',
      output_schema_id: 'schema-002',
    }
    const schema2: OutputSchema = {
      id: 'schema-002',
      name: 'FetchSchema',
      schema: {
        type: 'object',
        properties: {
          data: { type: 'string' },
        },
      },
      created_at: '2025-01-01T00:00:00Z',
    }
    const edges: WorkflowStepEdge[] = [
      edge,
      { id: 'edge-002', from_step_id: 'step-003', to_step_id: 'step-002' },
    ]
    const schemas = new Map([['schema-001', schema], ['schema-002', schema2]])

    const completions = buildVariableCompletions(
      'step-002',
      new Map([['step-001', baseStep], ['step-002', currentStep], ['step-003', step3]]),
      edges,
      schemas,
    )

    const sections = [...new Set(completions.map((c) => c.section))]
    expect(sections).toEqual(['Parse Input', 'Fetch Data'])
  })

  it('uses execution_mode as fallback when step name is null', () => {
    const unnamedStep: WorkflowStep = { ...baseStep, name: null }
    const schemas = new Map([['schema-001', schema]])
    const completions = buildVariableCompletions(
      'step-002',
      new Map([['step-001', unnamedStep], ['step-002', currentStep]]),
      [edge],
      schemas,
    )

    expect(completions[0]?.section).toBe('single')
    expect(completions[0]?.detail).toContain('single')
  })
})
