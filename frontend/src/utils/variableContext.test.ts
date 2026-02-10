import { describe, it, expect } from 'vitest'
import { buildVariableCompletions, toSnakeCase } from './variableContext'
import type { WorkflowStep } from '@/types/workflow'
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

const nestedArraySchema: OutputSchema = {
  id: 'schema-array',
  name: 'ArraySchema',
  schema: {
    type: 'object',
    properties: {
      items: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            name: { type: 'string' },
            score: { type: 'number' },
          },
        },
      },
      title: { type: 'string' },
    },
  },
  created_at: '2025-01-01T00:00:00Z',
}

const rootArraySchema: OutputSchema = {
  id: 'schema-root-array',
  name: 'RootArraySchema',
  schema: {
    type: 'array',
    items: {
      type: 'object',
      additionalProperties: false,
      properties: {
        content: { type: 'object', description: 'The task content' },
        port: { type: 'string', description: 'The target agent port' },
      },
      required: ['port', 'content'],
    },
  },
  created_at: '2025-01-01T00:00:00Z',
}

describe('toSnakeCase', () => {
  it('converts step names to snake_case', () => {
    expect(toSnakeCase('Research Agent')).toBe('research_agent')
    expect(toSnakeCase('Write a Summery Report')).toBe('write_a_summery_report')
    expect(toSnakeCase('Parse Input')).toBe('parse_input')
    expect(toSnakeCase('single')).toBe('single')
  })

  it('handles special characters', () => {
    expect(toSnakeCase('  Leading Spaces  ')).toBe('leading_spaces')
    expect(toSnakeCase('with-dashes')).toBe('with_dashes')
    expect(toSnakeCase('with.dots')).toBe('with_dots')
  })
})

describe('buildVariableCompletions', () => {
  it('builds completions from upstream step with output schema', () => {
    const schemas = new Map([['schema-001', schema]])
    const { completions } = buildVariableCompletions(
      ['step-001'],
      new Map([
        ['step-001', baseStep],
        ['step-002', currentStep],
      ]),
      schemas,
      null,
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

  it('auto-derives variable name from step name when output_variable_name is null', () => {
    const stepNoVar: WorkflowStep = {
      ...baseStep,
      name: 'Research Agent',
      output_variable_name: null,
    }
    const schemas = new Map([['schema-001', schema]])
    const { completions, autoNamed } = buildVariableCompletions(
      ['step-001'],
      new Map([
        ['step-001', stepNoVar],
        ['step-002', currentStep],
      ]),
      schemas,
      null,
    )

    expect(completions[0]?.label).toBe('{research_agent}')
    expect(completions.length).toBeGreaterThan(1)
    expect(autoNamed).toEqual([{ stepId: 'step-001', derivedName: 'research_agent' }])
  })

  it('shows root variable without schema fields when no schema assigned', () => {
    const stepNoSchema: WorkflowStep = { ...baseStep, output_schema_id: null }
    const schemas = new Map([['schema-001', schema]])
    const { completions } = buildVariableCompletions(
      ['step-001'],
      new Map([
        ['step-001', stepNoSchema],
        ['step-002', currentStep],
      ]),
      schemas,
      null,
    )

    expect(completions).toEqual([
      {
        label: '{parse_output}',
        displayLabel: 'parse_output',
        detail: 'any \u2014 from Parse Input',
        section: 'Parse Input',
      },
    ])
  })

  it('returns empty when no upstream IDs', () => {
    const schemas = new Map([['schema-001', schema]])
    const { completions, autoNamed } = buildVariableCompletions(
      [],
      new Map([
        ['step-001', baseStep],
        ['step-002', currentStep],
      ]),
      schemas,
      null,
    )

    expect(completions).toEqual([])
    expect(autoNamed).toEqual([])
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
    const schemas = new Map([
      ['schema-001', schema],
      ['schema-002', schema2],
    ])

    const { completions } = buildVariableCompletions(
      ['step-001', 'step-003'],
      new Map([
        ['step-001', baseStep],
        ['step-002', currentStep],
        ['step-003', step3],
      ]),
      schemas,
      null,
    )

    const sections = [...new Set(completions.map((c) => c.section))]
    expect(sections).toEqual(['Parse Input', 'Fetch Data'])
  })

  it('uses execution_mode as fallback when step name is null', () => {
    const unnamedStep: WorkflowStep = { ...baseStep, name: null }
    const schemas = new Map([['schema-001', schema]])
    const { completions } = buildVariableCompletions(
      ['step-001'],
      new Map([
        ['step-001', unnamedStep],
        ['step-002', currentStep],
      ]),
      schemas,
      null,
    )

    expect(completions[0]?.section).toBe('single')
    expect(completions[0]?.detail).toContain('single')
  })

  it('does not include autoNamed entry when output_variable_name is already set', () => {
    const schemas = new Map([['schema-001', schema]])
    const { autoNamed } = buildVariableCompletions(
      ['step-001'],
      new Map([
        ['step-001', baseStep],
        ['step-002', currentStep],
      ]),
      schemas,
      null,
    )

    expect(autoNamed).toEqual([])
  })

  // ── Nested array .$ syntax ──────────────────────────────────────────

  it('generates .$ element chips for nested array-type fields', () => {
    const arrayStep: WorkflowStep = {
      ...baseStep,
      id: 'step-arr',
      name: 'Lister',
      output_variable_name: 'lister',
      output_schema_id: 'schema-array',
    }
    const schemas = new Map([['schema-array', nestedArraySchema]])
    const { completions } = buildVariableCompletions(['step-arr'], new Map([['step-arr', arrayStep]]), schemas, null)

    const labels = completions.map((c) => c.label)
    expect(labels).toContain('{lister.items.$}')
    expect(labels).toContain('{lister.items.$.name}')
    expect(labels).toContain('{lister.items.$.score}')
  })

  it('does not generate .$ chip for non-array fields', () => {
    const schemas = new Map([['schema-001', schema]])
    const { completions } = buildVariableCompletions(['step-001'], new Map([['step-001', baseStep]]), schemas, null)

    const labels = completions.map((c) => c.label)
    expect(labels).not.toContain('{parse_output.summary.$}')
    expect(labels).not.toContain('{parse_output.count.$}')
  })

  // ── Root-level array schema ─────────────────────────────────────────

  it('generates .$ element chips for root-level array schema', () => {
    const decompStep: WorkflowStep = {
      ...baseStep,
      id: 'step-decomp',
      name: 'Decomposition',
      output_variable_name: 'decomposition',
      output_schema_id: 'schema-root-array',
    }
    const schemas = new Map([['schema-root-array', rootArraySchema]])
    const { completions } = buildVariableCompletions(
      ['step-decomp'],
      new Map([
        ['step-decomp', decompStep],
        ['step-002', currentStep],
      ]),
      schemas,
      null,
    )

    const labels = completions.map((c) => c.label)
    expect(labels).toContain('{decomposition}')
    expect(labels).toContain('{decomposition.$}')
    expect(labels).toContain('{decomposition.$.content}')
    expect(labels).toContain('{decomposition.$.port}')
    // Should NOT have direct field access (invalid on array root)
    expect(labels).not.toContain('{decomposition.content}')
    expect(labels).not.toContain('{decomposition.port}')
  })

  it('shows root type as array for root-level array schema', () => {
    const decompStep: WorkflowStep = {
      ...baseStep,
      id: 'step-decomp',
      name: 'Decomposition',
      output_variable_name: 'decomposition',
      output_schema_id: 'schema-root-array',
    }
    const schemas = new Map([['schema-root-array', rootArraySchema]])
    const { completions } = buildVariableCompletions(
      ['step-decomp'],
      new Map([
        ['step-decomp', decompStep],
        ['step-002', currentStep],
      ]),
      schemas,
      null,
    )

    expect(completions[0]?.detail).toBe('array \u2014 from Decomposition')
  })
})
