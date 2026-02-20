import { describe, it, expect } from 'vitest'
import { toRFNodes, toRFEdges, toAgentEdges, nodeDataEqual, computeProtocolGroups } from '.'
import type { StepNodeLookups, ProtocolStepInfo } from '.'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import type { NodePalette } from '@/theme'

/** Test palette — uses Midnight values for backward-compat with existing assertions. */
const testPalette: NodePalette = {
  workforce: '#3b82f6',
  room: '#a78bfa',
  blank: '#7d8590',
  agent: '#06b6d4',
  context: '#10b981',
  input: '#f59e0b',
  step: '#7d8590',
  sub_workflow: '#10b981',
}

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
  rosterByStep: {},
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
      type: 'canvasNode',
      position: { x: 100, y: 200 },
      data: {
        variant: 'step',
        kind: 'step',
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

  it('maps WorkflowStepEdge array to React Flow edges with source color', () => {
    const edges = toRFEdges([edge1], emptyGroups, emptyProtocols, [step1, step2], testPalette)

    expect(edges).toHaveLength(1)
    expect(edges[0]).toEqual({
      id: 'edge-001',
      type: 'stepEdge',
      source: 'step-001',
      target: 'step-002',
      data: { sourceColor: '#7d8590', isProtocolEdge: false },
    })
  })

  it('returns empty array for empty input', () => {
    expect(toRFEdges([], emptyGroups, emptyProtocols, [], testPalette)).toEqual([])
  })

  it('resolves sourceColor from protocol step type', () => {
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['step-001', { protocol_type: 'workforce', name: 'Doc', portNames: [] }],
    ])
    const edges = toRFEdges([edge1], emptyGroups, protocols, [step1, step2], testPalette)
    // step1 execution_mode is 'single' → variant 'step' → grey
    expect(edges[0]?.data?.sourceColor).toBe('#7d8590')
    expect(edges[0]?.data?.isProtocolEdge).toBe(true)
  })

  it('marks isProtocolEdge when target is a protocol step', () => {
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['step-002', { protocol_type: 'workforce', name: 'Doc', portNames: [] }],
    ])
    const edges = toRFEdges([edge1], emptyGroups, protocols, [step1, step2], testPalette)
    // step1 execution_mode is 'single' → variant 'step' → grey
    expect(edges[0]?.data?.sourceColor).toBe('#7d8590')
    expect(edges[0]?.data?.isProtocolEdge).toBe(true)
  })

  it('uses intrinsic step color even when in a protocol group', () => {
    const groups = new Map([
      ['step-001', { protocolColor: '#3b82f6', protocolStepId: 'proto-1' }],
    ])
    const edges = toRFEdges([edge1], groups, emptyProtocols, [step1, step2], testPalette)
    // Source is a 'single' step → resolveVariant maps to 'step' variant
    expect(edges[0]?.data?.sourceColor).toBe('#7d8590')
    expect(edges[0]?.data?.isProtocolEdge).toBe(true)
  })

  it('resolves sourceColor for workforce steps by execution_mode', () => {
    const workforceA: WorkflowStep = { ...step1, id: 'doc-a', execution_mode: 'workforce' }
    const workforceB: WorkflowStep = { ...step1, id: 'doc-b', execution_mode: 'workforce' }
    const edge: WorkflowStepEdge = { id: 'edge-doc', from_step_id: 'doc-a', to_step_id: 'doc-b' }
    const edges = toRFEdges([edge], emptyGroups, emptyProtocols, [workforceA, workforceB], testPalette)
    expect(edges[0]?.data?.sourceColor).toBe('#3b82f6')
    expect(edges[0]?.data?.isProtocolEdge).toBe(true)
  })

  it('resolves sourceColor from step type for non-protocol edges', () => {
    const forEachStep: WorkflowStep = { ...step1, id: 'fe-1', execution_mode: 'for_each' }
    const edge: WorkflowStepEdge = { id: 'edge-fe', from_step_id: 'fe-1', to_step_id: 'step-002' }
    const edges = toRFEdges([edge], emptyGroups, emptyProtocols, [forEachStep, step2], testPalette)
    // for_each → variant 'step' → grey
    expect(edges[0]?.data?.sourceColor).toBe('#7d8590')
    expect(edges[0]?.data?.isProtocolEdge).toBe(false)
  })

  it('uses default target handle for Input→Protocol edges', () => {
    const inputStep: WorkflowStep = { ...step1, id: 'input-1', execution_mode: 'input' }
    const protocolStep: WorkflowStep = { ...step1, id: 'proto-1', execution_mode: 'workforce' }
    const edge: WorkflowStepEdge = { id: 'edge-input', from_step_id: 'input-1', to_step_id: 'proto-1' }
    const edges = toRFEdges([edge], emptyGroups, emptyProtocols, [inputStep, protocolStep], testPalette)
    expect(edges[0]?.targetHandle).toBeUndefined()
  })

  it('uses default target handle for Context→Protocol edges', () => {
    const ctxStep: WorkflowStep = { ...step1, id: 'ctx-1', execution_mode: 'context' }
    const protocolStep: WorkflowStep = { ...step1, id: 'proto-1', execution_mode: 'workforce' }
    const edge: WorkflowStepEdge = { id: 'edge-ctx', from_step_id: 'ctx-1', to_step_id: 'proto-1' }
    const edges = toRFEdges([edge], emptyGroups, emptyProtocols, [ctxStep, protocolStep], testPalette)
    expect(edges[0]?.targetHandle).toBeUndefined()
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
    const steps = [makeStep('proto', 'workforce'), makeStep('s1')]
    const edges = [{ from_step_id: 's1', to_step_id: 'proto' }]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['proto', { protocol_type: 'workforce', name: 'Doc', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)
    expect(result.get('s1')).toEqual({ protocolColor: '#3b82f6', protocolStepId: 'proto' })
  })

  it('does not include the protocol step itself in the group', () => {
    const steps = [makeStep('proto', 'workforce'), makeStep('s1')]
    const edges = [{ from_step_id: 's1', to_step_id: 'proto' }]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['proto', { protocol_type: 'workforce', name: 'Doc', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)
    expect(result.has('proto')).toBe(false)
  })

  it('colors nodes reachable through intermediate nodes', () => {
    const steps = [makeStep('proto', 'workforce'), makeStep('s1'), makeStep('s2')]
    const edges = [
      { from_step_id: 's1', to_step_id: 'proto' },
      { from_step_id: 's2', to_step_id: 's1' },
    ]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['proto', { protocol_type: 'workforce', name: 'Doc', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)
    expect(result.get('s1')?.protocolStepId).toBe('proto')
    expect(result.get('s2')?.protocolStepId).toBe('proto')
  })

  it('leaves disconnected nodes out of the group', () => {
    const steps = [makeStep('proto', 'workforce'), makeStep('s1'), makeStep('s2')]
    const edges = [{ from_step_id: 's1', to_step_id: 'proto' }]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['proto', { protocol_type: 'workforce', name: 'Doc', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)
    expect(result.has('s1')).toBe(true)
    expect(result.has('s2')).toBe(false)
  })

  it('detects protocol by execution_mode even without stepProtocols entry', () => {
    const steps = [makeStep('proto', 'workforce'), makeStep('s1')]
    const edges = [{ from_step_id: 's1', to_step_id: 'proto' }]
    const result = computeProtocolGroups(steps, edges, new Map())
    expect(result.get('s1')?.protocolStepId).toBe('proto')
  })

  it('does not traverse through a second protocol into its neighbors', () => {
    // Context → Workforce A → Workforce B
    // Context should belong to A only, not B
    const steps = [
      makeStep('context-1', 'context'),
      makeStep('workforce-A', 'workforce'),
      makeStep('workforce-B', 'workforce'),
    ]
    const edges = [
      { from_step_id: 'context-1', to_step_id: 'workforce-A' },
      { from_step_id: 'workforce-A', to_step_id: 'workforce-B' },
    ]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['workforce-A', { protocol_type: 'workforce', name: 'Doc A', portNames: [] }],
      ['workforce-B', { protocol_type: 'workforce', name: 'Doc B', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)

    expect(result.get('context-1')?.protocolStepId).toBe('workforce-A')
  })

  it('isolates groups when two protocols share a connected component', () => {
    // Context → Workforce A ← Step → Workforce B ← External
    // Context belongs to A, External belongs to B, Step belongs to whichever BFS finds it first
    const steps = [
      makeStep('context-1', 'context'),
      makeStep('workforce-A', 'workforce'),
      makeStep('step-mid', 'single'),
      makeStep('workforce-B', 'workforce'),
      makeStep('external', 'single'),
    ]
    const edges = [
      { from_step_id: 'context-1', to_step_id: 'workforce-A' },
      { from_step_id: 'step-mid', to_step_id: 'workforce-A' },
      { from_step_id: 'step-mid', to_step_id: 'workforce-B' },
      { from_step_id: 'external', to_step_id: 'workforce-B' },
    ]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['workforce-A', { protocol_type: 'workforce', name: 'Doc A', portNames: [] }],
      ['workforce-B', { protocol_type: 'workforce', name: 'Doc B', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)

    expect(result.get('context-1')?.protocolStepId).toBe('workforce-A')
    expect(result.get('external')?.protocolStepId).toBe('workforce-B')
  })

  it('does not let BFS from protocol B overwrite nodes belonging to protocol A', () => {
    // Context → Workforce A → Step → Workforce B
    // Without boundary fix, BFS from B would walk through A and claim Context
    const steps = [
      makeStep('context-1', 'context'),
      makeStep('workforce-A', 'workforce'),
      makeStep('step-between', 'single'),
      makeStep('workforce-B', 'workforce'),
    ]
    const edges = [
      { from_step_id: 'context-1', to_step_id: 'workforce-A' },
      { from_step_id: 'workforce-A', to_step_id: 'step-between' },
      { from_step_id: 'step-between', to_step_id: 'workforce-B' },
    ]
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['workforce-A', { protocol_type: 'workforce', name: 'Doc A', portNames: [] }],
      ['workforce-B', { protocol_type: 'workforce', name: 'Doc B', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)

    expect(result.get('context-1')?.protocolStepId).toBe('workforce-A')
    expect(result.get('step-between')?.protocolStepId).not.toBe(undefined)
    expect(result.has('workforce-A')).toBe(false)
    expect(result.has('workforce-B')).toBe(false)
  })
})

describe('toRFNodes — input nodes', () => {
  it('maps input step to inputNode with correct data', () => {
    const inputStep: WorkflowStep = {
      ...step1,
      id: 'input-001',
      name: 'User Input',
      execution_mode: 'input',
      prompt_template: 'Project contains 2 categories.',
      position_x: 50,
      position_y: 75,
    }
    const nodes = toRFNodes([inputStep], emptyLookups)

    expect(nodes).toHaveLength(1)
    expect(nodes[0]).toEqual({
      id: 'input-001',
      type: 'canvasNode',
      position: { x: 50, y: 75 },
      style: { width: 560, height: 500 },
      data: {
        variant: 'input',
        kind: 'input',
        label: 'User Input',
        content: 'Project contains 2 categories.',
        protocolColor: null,
        protocolStepId: null,
      },
    })
  })

  it('uses custom width/height from step when set', () => {
    const inputStep: WorkflowStep = {
      ...step1,
      id: 'input-002',
      execution_mode: 'input',
      width: 600,
      height: 500,
    }
    const nodes = toRFNodes([inputStep], emptyLookups)
    expect(nodes[0]?.style).toEqual({ width: 600, height: 500 })
  })

  it('falls back to "Input" when name is null', () => {
    const inputStep: WorkflowStep = {
      ...step1,
      id: 'input-003',
      execution_mode: 'input',
      name: null,
    }
    const nodes = toRFNodes([inputStep], emptyLookups)
    expect(nodes[0]?.data.label).toBe('Input')
  })
})

describe('toRFNodes — agent nodes', () => {
  const workforceStep: WorkflowStep = {
    ...step1,
    id: 'wf-step',
    execution_mode: 'workforce',
    position_x: 200,
    position_y: 300,
  }

  it('generates agent nodes for workforce steps with roster agents', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      rosterByStep: {
        'wf-step': [
          { id: 'agent-1', name: 'Researcher', child_step_id: 'cs-1', role_description: 'Does research', depends_on: [] },
          { id: 'agent-2', name: 'Writer', child_step_id: 'cs-2', role_description: 'Writes docs', depends_on: ['agent-1'] },
        ],
      },
      protocolsByStep: new Map([['wf-step', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
    }
    const nodes = toRFNodes([workforceStep], lookups)
    const agentNode1 = nodes.find((n) => n.id === 'agent-artifact-agent-1')
    const agentNode2 = nodes.find((n) => n.id === 'agent-artifact-agent-2')

    expect(agentNode1).toBeDefined()
    expect(agentNode1?.type).toBe('canvasNode')
    expect(agentNode1?.data.kind).toBe('agent')
    expect(agentNode1?.data.label).toBe('Researcher')
    expect(agentNode1?.data.roleDescription).toBe('Does research')
    expect(agentNode1?.data.protocolStepId).toBe('wf-step')
    expect(agentNode1?.connectable).toBe(false)
    expect(agentNode1?.draggable).toBe(true)

    expect(agentNode2).toBeDefined()
    expect(agentNode2?.data.label).toBe('Writer')
  })

  it('skips agents without child_step_id', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      rosterByStep: {
        'wf-step': [
          { id: 'agent-1', name: 'Old Agent', child_step_id: null, role_description: '', depends_on: [] },
        ],
      },
      protocolsByStep: new Map([['wf-step', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
    }
    const nodes = toRFNodes([workforceStep], lookups)
    const agentNode = nodes.find((n) => n.id === 'agent-artifact-agent-1')
    expect(agentNode).toBeUndefined()
  })

  it('does not generate agent nodes for non-workforce steps', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      rosterByStep: {
        'step-001': [
          { id: 'agent-1', name: 'Agent', child_step_id: 'cs-1', role_description: '', depends_on: [] },
        ],
      },
    }
    const nodes = toRFNodes([step1], lookups)
    const agentNode = nodes.find((n) => n.id === 'agent-artifact-agent-1')
    expect(agentNode).toBeUndefined()
  })
})

describe('toAgentEdges', () => {
  const workforceStep: WorkflowStep = {
    ...step1,
    id: 'wf-step',
    execution_mode: 'workforce',
  }

  it('generates protocol-to-agent edges for roster agents', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      rosterByStep: {
        'wf-step': [
          { id: 'agent-1', name: 'Researcher', child_step_id: 'cs-1', role_description: '', depends_on: [] },
        ],
      },
      protocolsByStep: new Map([['wf-step', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
    }
    const edges = toAgentEdges([workforceStep], lookups, testPalette)
    expect(edges).toHaveLength(1)
    expect(edges[0]).toEqual({
      id: 'agent-edge-agent-1',
      type: 'artifactEdge',
      data: { color: '#06b6d4' },
      source: 'wf-step',
      sourceHandle: 'agents',
      target: 'agent-artifact-agent-1',
      targetHandle: 'agent-input',
      selectable: false,
      deletable: false,
    })
  })

  it('fans out root agents from protocol, uses depends_on for non-roots', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      rosterByStep: {
        'wf-step': [
          { id: 'agent-1', name: 'Researcher', child_step_id: 'cs-1', role_description: '', depends_on: [] },
          { id: 'agent-2', name: 'Writer', child_step_id: 'cs-2', role_description: '', depends_on: [] },
          { id: 'agent-3', name: 'Judge', child_step_id: 'cs-3', role_description: '', depends_on: ['agent-1', 'agent-2'] },
        ],
      },
      protocolsByStep: new Map([['wf-step', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
    }
    const edges = toAgentEdges([workforceStep], lookups, testPalette)
    // 2 root edges + 2 dependency edges = 4
    expect(edges).toHaveLength(4)
    // Root agents fan from protocol
    expect(edges[0]).toEqual({
      id: 'agent-edge-agent-1',
      type: 'artifactEdge',
      data: { color: '#06b6d4' },
      source: 'wf-step',
      sourceHandle: 'agents',
      target: 'agent-artifact-agent-1',
      targetHandle: 'agent-input',
      selectable: false,
      deletable: false,
    })
    expect(edges[1]).toEqual({
      id: 'agent-edge-agent-2',
      type: 'artifactEdge',
      data: { color: '#06b6d4' },
      source: 'wf-step',
      sourceHandle: 'agents',
      target: 'agent-artifact-agent-2',
      targetHandle: 'agent-input',
      selectable: false,
      deletable: false,
    })
    // Judge depends on both agents — dependency edges
    expect(edges[2]).toEqual({
      id: 'agent-dep-agent-1-agent-3',
      type: 'artifactEdge',
      data: { color: '#06b6d4' },
      source: 'agent-artifact-agent-1',
      sourceHandle: 'agent-output',
      target: 'agent-artifact-agent-3',
      targetHandle: 'agent-input',
      selectable: false,
      deletable: false,
    })
    expect(edges[3]).toEqual({
      id: 'agent-dep-agent-2-agent-3',
      type: 'artifactEdge',
      data: { color: '#06b6d4' },
      source: 'agent-artifact-agent-2',
      sourceHandle: 'agent-output',
      target: 'agent-artifact-agent-3',
      targetHandle: 'agent-input',
      selectable: false,
      deletable: false,
    })
  })

  it('skips agents without child_step_id', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      rosterByStep: {
        'wf-step': [
          { id: 'agent-1', name: 'Old', child_step_id: null, role_description: '', depends_on: [] },
        ],
      },
      protocolsByStep: new Map([['wf-step', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
    }
    const edges = toAgentEdges([workforceStep], lookups, testPalette)
    expect(edges).toEqual([])
  })

  it('returns empty for non-workforce steps', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      rosterByStep: {
        'step-001': [
          { id: 'agent-1', name: 'Agent', child_step_id: 'cs-1', role_description: '', depends_on: [] },
        ],
      },
    }
    const edges = toAgentEdges([step1], lookups, testPalette)
    expect(edges).toEqual([])
  })
})

describe('toRFNodes — agent vertical stacking', () => {
  const workforceStep: WorkflowStep = {
    ...step1,
    id: 'wf-step',
    execution_mode: 'workforce',
    position_x: 200,
    position_y: 300,
  }

  it('stacks agents vertically above the protocol node', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      rosterByStep: {
        'wf-step': [
          { id: 'agent-1', name: 'First', child_step_id: 'cs-1', role_description: '', depends_on: [] },
          { id: 'agent-2', name: 'Second', child_step_id: 'cs-2', role_description: '', depends_on: [] },
        ],
      },
      protocolsByStep: new Map([['wf-step', { protocol_type: 'workforce', name: 'Team', portNames: [] }]]),
    }
    const nodes = toRFNodes([workforceStep], lookups)
    const agent1 = nodes.find((n) => n.id === 'agent-artifact-agent-1')
    const agent2 = nodes.find((n) => n.id === 'agent-artifact-agent-2')

    // Same x as protocol (vertical column)
    expect(agent1?.position.x).toBe(200)
    expect(agent2?.position.x).toBe(200)

    // Stacked above protocol, agent2 higher than agent1
    expect(agent1?.position.y).toBeLessThan(300)
    expect(agent2?.position.y).toBeLessThan(agent1!.position.y)
  })
})

