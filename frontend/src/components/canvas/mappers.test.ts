import { describe, it, expect } from 'vitest'
import { toRFNodes, toRFEdges, nodeDataEqual, computeProtocolGroups } from './mappers'
import type { StepNodeLookups, ProtocolStepInfo } from './mappers'
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
  toolsByAgent: new Map(),
  protocolsByStep: new Map(),
  documentDefsByStep: {},
  protocolGroups: new Map(),
}

describe('toRFNodes', () => {
  it('maps WorkflowStep array to React Flow nodes', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      stepNames: new Map([
        ['step-001', 'First Step'],
        ['step-002', 'Second Step'],
      ]),
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
        toolNames: [],
        protocolType: null,
        protocolName: null,
        protocolPortNames: [],
        protocolColor: null,
        protocolStepId: null,
        isProtocol: false,
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
      stepNames: new Map([
        ['step-001', 'First Step'],
        ['step-002', 'Second Step'],
      ]),
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
  const emptyGroups = new Map()
  const emptyProtocols: ReadonlyMap<string, ProtocolStepInfo> = new Map()

  it('maps WorkflowStepEdge array to React Flow edges', () => {
    const edges = toRFEdges([edge1], emptyGroups, emptyProtocols)

    expect(edges).toHaveLength(1)
    expect(edges[0]).toEqual({
      id: 'edge-001',
      type: 'stepEdge',
      source: 'step-001',
      target: 'step-002',
      data: { protocolColor: null },
    })
  })

  it('returns empty array for empty input', () => {
    expect(toRFEdges([], emptyGroups, emptyProtocols)).toEqual([])
  })

  it('sets protocolColor when source is a protocol step', () => {
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['step-001', { protocol_type: 'documenter', name: 'Doc', portNames: [] }],
    ])
    const edges = toRFEdges([edge1], emptyGroups, protocols)
    expect(edges[0]?.data?.protocolColor).toBe('#D4793E')
  })

  it('sets protocolColor when target is a protocol step', () => {
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['step-002', { protocol_type: 'documenter', name: 'Doc', portNames: [] }],
    ])
    const edges = toRFEdges([edge1], emptyGroups, protocols)
    expect(edges[0]?.data?.protocolColor).toBe('#D4793E')
  })

  it('sets protocolColor from protocol group membership', () => {
    const groups = new Map([
      ['step-001', { protocolColor: '#D4793E', protocolStepId: 'proto-1' }],
    ])
    const edges = toRFEdges([edge1], groups, emptyProtocols)
    expect(edges[0]?.data?.protocolColor).toBe('#D4793E')
  })
})

describe('nodeDataEqual', () => {
  it('returns true for identical primitive-only objects', () => {
    const a = { label: 'Step', agentId: 'a1', modelId: null }
    const b = { label: 'Step', agentId: 'a1', modelId: null }
    expect(nodeDataEqual(a, b)).toBe(true)
  })

  it('returns false when a primitive value differs', () => {
    const a = { label: 'Step', agentId: 'a1' }
    const b = { label: 'Step', agentId: 'a2' }
    expect(nodeDataEqual(a, b)).toBe(false)
  })

  it('returns false when key count differs', () => {
    const a = { label: 'Step' }
    const b = { label: 'Step', extra: true }
    expect(nodeDataEqual(a, b)).toBe(false)
  })

  it('compares array values element-wise', () => {
    const a = { names: ['x', 'y'] }
    const b = { names: ['x', 'y'] }
    expect(nodeDataEqual(a, b)).toBe(true)
  })

  it('returns false when array values differ', () => {
    const a = { names: ['x', 'y'] }
    const b = { names: ['x', 'z'] }
    expect(nodeDataEqual(a, b)).toBe(false)
  })

  it('returns false when one value is array and other is not', () => {
    const a = { v: ['x'] }
    const b = { v: 'x' }
    expect(nodeDataEqual(a, b)).toBe(false)
  })

  it('compares empty objects as equal', () => {
    expect(nodeDataEqual({}, {})).toBe(true)
  })

  it('compares empty arrays as equal', () => {
    const a = { items: [] as unknown[] }
    const b = { items: [] as unknown[] }
    expect(nodeDataEqual(a, b)).toBe(true)
  })

  it('handles StepNodeData shape', () => {
    const data = {
      label: 'Review',
      stepType: 'single',
      agentId: 'a1',
      promptTemplateId: null,
      outputSchemaId: null,
      agentName: 'Bot',
      modelId: 'claude-sonnet-4',
      outputSchemaName: null,
      upstreamStepNames: ['Plan'],
      toolNames: ['grep', 'read'],
      protocolType: null,
      protocolName: null,
      protocolPortNames: [],
      protocolColor: null,
      protocolStepId: null,
      isProtocol: false,
    }
    const clone = { ...data, upstreamStepNames: ['Plan'], toolNames: ['grep', 'read'], protocolPortNames: [] }
    expect(nodeDataEqual(data, clone)).toBe(true)
  })
})

describe('computeProtocolGroups', () => {
  const makeStep = (id: string, mode = 'single'): WorkflowStep => ({
    ...step1,
    id,
    execution_mode: mode,
  })

  it('returns empty map when no protocols exist', () => {
    const steps = [makeStep('s1'), makeStep('s2')]
    const edges = [{ from_step_id: 's1', to_step_id: 's2' }]
    const result = computeProtocolGroups(steps, edges, new Map())
    expect(result.size).toBe(0)
  })

  it('assigns protocol color to directly connected nodes', () => {
    const steps = [makeStep('proto', 'documenter'), makeStep('s1')]
    const edges = [{ from_step_id: 's1', to_step_id: 'proto' }]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['proto', { protocol_type: 'documenter', name: 'Doc', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)
    expect(result.get('s1')).toEqual({ protocolColor: '#D4793E', protocolStepId: 'proto' })
  })

  it('does not include the protocol step itself in the group', () => {
    const steps = [makeStep('proto', 'documenter'), makeStep('s1')]
    const edges = [{ from_step_id: 's1', to_step_id: 'proto' }]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['proto', { protocol_type: 'documenter', name: 'Doc', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)
    expect(result.has('proto')).toBe(false)
  })

  it('colors nodes reachable through intermediate nodes', () => {
    const steps = [makeStep('proto', 'documenter'), makeStep('s1'), makeStep('s2')]
    const edges = [
      { from_step_id: 's1', to_step_id: 'proto' },
      { from_step_id: 's2', to_step_id: 's1' },
    ]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['proto', { protocol_type: 'documenter', name: 'Doc', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)
    expect(result.get('s1')?.protocolStepId).toBe('proto')
    expect(result.get('s2')?.protocolStepId).toBe('proto')
  })

  it('leaves disconnected nodes out of the group', () => {
    const steps = [makeStep('proto', 'documenter'), makeStep('s1'), makeStep('s2')]
    const edges = [{ from_step_id: 's1', to_step_id: 'proto' }]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['proto', { protocol_type: 'documenter', name: 'Doc', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)
    expect(result.has('s1')).toBe(true)
    expect(result.has('s2')).toBe(false)
  })

  it('detects protocol by execution_mode even without stepProtocols entry', () => {
    const steps = [makeStep('proto', 'documenter'), makeStep('s1')]
    const edges = [{ from_step_id: 's1', to_step_id: 'proto' }]
    const result = computeProtocolGroups(steps, edges, new Map())
    expect(result.get('s1')?.protocolStepId).toBe('proto')
  })

  it('does not traverse through a second protocol into its neighbors', () => {
    // Context → Documenter A → Documenter B
    // Context should belong to A only, not B
    const steps = [
      makeStep('context-1', 'context'),
      makeStep('documenter-A', 'documenter'),
      makeStep('documenter-B', 'documenter'),
    ]
    const edges = [
      { from_step_id: 'context-1', to_step_id: 'documenter-A' },
      { from_step_id: 'documenter-A', to_step_id: 'documenter-B' },
    ]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['documenter-A', { protocol_type: 'documenter', name: 'Doc A', portNames: [] }],
      ['documenter-B', { protocol_type: 'documenter', name: 'Doc B', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)

    expect(result.get('context-1')?.protocolStepId).toBe('documenter-A')
  })

  it('isolates groups when two protocols share a connected component', () => {
    // Context → Documenter A ← Step → Documenter B ← External
    // Context belongs to A, External belongs to B, Step belongs to whichever BFS finds it first
    const steps = [
      makeStep('context-1', 'context'),
      makeStep('documenter-A', 'documenter'),
      makeStep('step-mid', 'single'),
      makeStep('documenter-B', 'documenter'),
      makeStep('external', 'single'),
    ]
    const edges = [
      { from_step_id: 'context-1', to_step_id: 'documenter-A' },
      { from_step_id: 'step-mid', to_step_id: 'documenter-A' },
      { from_step_id: 'step-mid', to_step_id: 'documenter-B' },
      { from_step_id: 'external', to_step_id: 'documenter-B' },
    ]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['documenter-A', { protocol_type: 'documenter', name: 'Doc A', portNames: [] }],
      ['documenter-B', { protocol_type: 'documenter', name: 'Doc B', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)

    expect(result.get('context-1')?.protocolStepId).toBe('documenter-A')
    expect(result.get('external')?.protocolStepId).toBe('documenter-B')
  })

  it('does not let BFS from protocol B overwrite nodes belonging to protocol A', () => {
    // Context → Documenter A → Step → Documenter B
    // Without boundary fix, BFS from B would walk through A and claim Context
    const steps = [
      makeStep('context-1', 'context'),
      makeStep('documenter-A', 'documenter'),
      makeStep('step-between', 'single'),
      makeStep('documenter-B', 'documenter'),
    ]
    const edges = [
      { from_step_id: 'context-1', to_step_id: 'documenter-A' },
      { from_step_id: 'documenter-A', to_step_id: 'step-between' },
      { from_step_id: 'step-between', to_step_id: 'documenter-B' },
    ]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['documenter-A', { protocol_type: 'documenter', name: 'Doc A', portNames: [] }],
      ['documenter-B', { protocol_type: 'documenter', name: 'Doc B', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)

    expect(result.get('context-1')?.protocolStepId).toBe('documenter-A')
    expect(result.get('step-between')?.protocolStepId).not.toBe(undefined)
    expect(result.has('documenter-A')).toBe(false)
    expect(result.has('documenter-B')).toBe(false)
  })
})
