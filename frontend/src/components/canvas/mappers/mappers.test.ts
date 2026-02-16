import { describe, it, expect } from 'vitest'
import { toRFNodes, toRFEdges, toNotesEdges, nodeDataEqual, computeProtocolGroups } from '.'
import type { StepNodeLookups, ProtocolStepInfo } from '.'
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
  documentContentByDefId: {},
  rosterByStep: {},
  notesByStep: {},
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
    const edges = toRFEdges([edge1], emptyGroups, emptyProtocols, [step1, step2])

    expect(edges).toHaveLength(1)
    expect(edges[0]).toEqual({
      id: 'edge-001',
      type: 'stepEdge',
      source: 'step-001',
      target: 'step-002',
      data: { sourceColor: '#3b82f6', isProtocolEdge: false },
    })
  })

  it('returns empty array for empty input', () => {
    expect(toRFEdges([], emptyGroups, emptyProtocols, [])).toEqual([])
  })

  it('resolves sourceColor from protocol step type', () => {
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['step-001', { protocol_type: 'workforce', name: 'Doc', portNames: [] }],
    ])
    const edges = toRFEdges([edge1], emptyGroups, protocols, [step1, step2])
    expect(edges[0]?.data?.sourceColor).toBe('#3b82f6')
    expect(edges[0]?.data?.isProtocolEdge).toBe(true)
  })

  it('marks isProtocolEdge when target is a protocol step', () => {
    const protocols: ReadonlyMap<string, ProtocolStepInfo> = new Map([
      ['step-002', { protocol_type: 'workforce', name: 'Doc', portNames: [] }],
    ])
    const edges = toRFEdges([edge1], emptyGroups, protocols, [step1, step2])
    expect(edges[0]?.data?.sourceColor).toBe('#3b82f6')
    expect(edges[0]?.data?.isProtocolEdge).toBe(true)
  })

  it('uses intrinsic step color even when in a protocol group', () => {
    const groups = new Map([
      ['step-001', { protocolColor: '#3b82f6', protocolStepId: 'proto-1' }],
    ])
    const edges = toRFEdges([edge1], groups, emptyProtocols, [step1, step2])
    // Source is a 'single' step → uses its own step type color, not the group color
    expect(edges[0]?.data?.sourceColor).toBe('#3b82f6')
    expect(edges[0]?.data?.isProtocolEdge).toBe(true)
  })

  it('resolves sourceColor for workforce steps by execution_mode', () => {
    const workforceA: WorkflowStep = { ...step1, id: 'doc-a', execution_mode: 'workforce' }
    const workforceB: WorkflowStep = { ...step1, id: 'doc-b', execution_mode: 'workforce' }
    const edge: WorkflowStepEdge = { id: 'edge-doc', from_step_id: 'doc-a', to_step_id: 'doc-b' }
    const edges = toRFEdges([edge], emptyGroups, emptyProtocols, [workforceA, workforceB])
    expect(edges[0]?.data?.sourceColor).toBe('#3b82f6')
    expect(edges[0]?.data?.isProtocolEdge).toBe(true)
  })

  it('resolves sourceColor from step type for non-protocol edges', () => {
    const forEachStep: WorkflowStep = { ...step1, id: 'fe-1', execution_mode: 'for_each' }
    const edge: WorkflowStepEdge = { id: 'edge-fe', from_step_id: 'fe-1', to_step_id: 'step-002' }
    const edges = toRFEdges([edge], emptyGroups, emptyProtocols, [forEachStep, step2])
    expect(edges[0]?.data?.sourceColor).toBe('#2dd4bf')
    expect(edges[0]?.data?.isProtocolEdge).toBe(false)
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

describe('toRFNodes — notes nodes', () => {
  it('generates a notes node when notesByStep has content', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      notesByStep: { 'step-001': '## Direction\n- Build auth' },
    }
    const nodes = toRFNodes([step1], lookups)
    const notesNode = nodes.find((n) => n.id === 'notes-step-001')
    expect(notesNode).toBeDefined()
    expect(notesNode?.type).toBe('notesNode')
    expect(notesNode?.data.kind).toBe('notes')
    expect(notesNode?.data.label).toBe('Agent Notes')
    expect(notesNode?.data.stepName).toBe('First Step')
    expect(notesNode?.data.content).toBe('## Direction\n- Build auth')
    expect(notesNode?.data.protocolStepId).toBe('step-001')
    expect(notesNode?.connectable).toBe(false)
    expect(notesNode?.draggable).toBe(true)
  })

  it('does not generate notes node when notesByStep is empty for the step', () => {
    const nodes = toRFNodes([step1], emptyLookups)
    const notesNode = nodes.find((n) => n.id === 'notes-step-001')
    expect(notesNode).toBeUndefined()
  })

  it('skips notes node for context steps', () => {
    const contextStep: WorkflowStep = { ...step1, id: 'ctx-1', execution_mode: 'context' }
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      notesByStep: { 'ctx-1': 'should not appear' },
    }
    const nodes = toRFNodes([contextStep], lookups)
    const notesNode = nodes.find((n) => n.id === 'notes-ctx-1')
    expect(notesNode).toBeUndefined()
  })

  it('skips notes node for input steps', () => {
    const inputStep: WorkflowStep = { ...step1, id: 'input-1', execution_mode: 'input' }
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      notesByStep: { 'input-1': 'should not appear' },
    }
    const nodes = toRFNodes([inputStep], lookups)
    const notesNode = nodes.find((n) => n.id === 'notes-input-1')
    expect(notesNode).toBeUndefined()
  })

  it('positions notes node to the left of the parent step', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      notesByStep: { 'step-001': 'some notes' },
    }
    const nodes = toRFNodes([step1], lookups)
    const notesNode = nodes.find((n) => n.id === 'notes-step-001')
    expect(notesNode?.position.x).toBe(step1.position_x ?? 0)
    expect(notesNode?.position.y).toBeGreaterThan(step1.position_y ?? 0)
  })

  it('falls back to execution_mode for stepName when name is null', () => {
    const stepNoName: WorkflowStep = { ...step1, name: null }
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      notesByStep: { 'step-001': 'notes' },
    }
    const nodes = toRFNodes([stepNoName], lookups)
    const notesNode = nodes.find((n) => n.id === 'notes-step-001')
    expect(notesNode?.data.stepName).toBe('single')
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
      type: 'inputNode',
      position: { x: 50, y: 75 },
      style: { width: 420, height: 360 },
      data: {
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

describe('toRFNodes — document nodes', () => {
  const workforceStep: WorkflowStep = {
    ...step1,
    id: 'doc-step',
    execution_mode: 'workforce',
    position_x: 200,
    position_y: 300,
  }

  it('generates document nodes with content from documentContentByDefId', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      documentDefsByStep: {
        'doc-step': [
          { id: 'def-1', step_id: 'doc-step', name: 'README', description: '', target_length: 5000, display_order: 0, created_at: '2025-01-01', document_id: 'doc-aaa' },
        ],
      },
      documentContentByDefId: { 'def-1': '# Generated README' },
      protocolsByStep: new Map([['doc-step', { protocol_type: 'workforce', name: 'Doc', portNames: [] }]]),
    }
    const nodes = toRFNodes([workforceStep], lookups)
    const docNode = nodes.find((n) => n.id === 'doc-artifact-def-1')
    expect(docNode).toBeDefined()
    expect(docNode?.type).toBe('documentNode')
    expect(docNode?.data.content).toBe('# Generated README')
  })

  it('falls back to empty string when no content exists for def', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      documentDefsByStep: {
        'doc-step': [
          { id: 'def-2', step_id: 'doc-step', name: 'CHANGELOG', description: '', target_length: 2000, display_order: 0, created_at: '2025-01-01', document_id: null },
        ],
      },
      protocolsByStep: new Map([['doc-step', { protocol_type: 'workforce', name: 'Doc', portNames: [] }]]),
    }
    const nodes = toRFNodes([workforceStep], lookups)
    const docNode = nodes.find((n) => n.id === 'doc-artifact-def-2')
    expect(docNode).toBeDefined()
    expect(docNode?.data.content).toBe('')
  })
})

describe('toNotesEdges', () => {
  it('generates an edge for each step with notes', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      notesByStep: { 'step-001': '## Notes' },
    }
    const edges = toNotesEdges([step1], lookups)
    expect(edges).toHaveLength(1)
    expect(edges[0]).toEqual({
      id: 'notes-edge-step-001',
      type: 'notesEdge',
      source: 'step-001',
      sourceHandle: 'notes',
      target: 'notes-step-001',
      targetHandle: 'notes-input',
      selectable: false,
      deletable: false,
    })
  })

  it('returns empty array when no notes exist', () => {
    const edges = toNotesEdges([step1, step2], emptyLookups)
    expect(edges).toEqual([])
  })

  it('skips context steps', () => {
    const contextStep: WorkflowStep = { ...step1, id: 'ctx-1', execution_mode: 'context' }
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      notesByStep: { 'ctx-1': 'hidden' },
    }
    const edges = toNotesEdges([contextStep], lookups)
    expect(edges).toEqual([])
  })

  it('skips input steps', () => {
    const inputStep: WorkflowStep = { ...step1, id: 'input-1', execution_mode: 'input' }
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      notesByStep: { 'input-1': 'hidden' },
    }
    const edges = toNotesEdges([inputStep], lookups)
    expect(edges).toEqual([])
  })

  it('generates edges only for steps that have notes', () => {
    const lookups: StepNodeLookups = {
      ...emptyLookups,
      notesByStep: { 'step-002': 'has notes' },
    }
    const edges = toNotesEdges([step1, step2], lookups)
    expect(edges).toHaveLength(1)
    expect(edges[0]?.source).toBe('step-002')
  })
})
