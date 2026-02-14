import { describe, it, expect, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useCanvasLookups } from './useCanvasLookups'
import type { WorkflowStep, WorkflowStepEdge, DocumentDef, RosterAgent } from '@/types/workflow'
import type { Agent, Tool } from '@/types'
import type { OutputSchema } from '@/types'
import type { StepProtocolLink } from '@/stores'

vi.mock('./mappers', () => ({
  computeProtocolGroups: vi.fn(() => new Map()),
}))

const makeStep = (id: string, mode = 'single', name: string | null = null): WorkflowStep => ({
  id,
  workflow_id: 'wf-1',
  agent_id: 'agent-1',
  execution_mode: mode,
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
  position_x: 0,
  position_y: 0,
  name,
  room_id: null,
  system_prompt_suffix: null,
  description: '',
})

const makeAgent = (id: string, name: string): Agent => ({
  id,
  name,
  system_prompt: '',
  model_provider: 'openai',
  model_id: 'gpt-4',
  model_max_tokens: 4096,
  model_temperature: 0.7,
  status: 'idle',
  output_schema_id: null,
  router_id: null,
  version: 1,
})

const makeSchema = (id: string, name: string): OutputSchema => ({
  id,
  name,
  description: '',
  json_schema: {},
  version: 1,
})

const makeEdge = (from: string, to: string): WorkflowStepEdge => ({
  id: `${from}-${to}`,
  from_step_id: from,
  to_step_id: to,
})

const emptyProtocols: Readonly<Record<string, StepProtocolLink>> = {}
const emptyDocDefs: Record<string, DocumentDef[]> = {}
const emptyRoster: Record<string, RosterAgent[]> = {}

describe('useCanvasLookups', () => {
  it('builds agent lookup map from agents', () => {
    const agents = [makeAgent('a1', 'Alice'), makeAgent('a2', 'Bob')]
    const { result } = renderHook(() =>
      useCanvasLookups([], [], agents, [], {}, emptyProtocols, emptyDocDefs, emptyRoster),
    )

    expect(result.current.lookups.agents.get('a1')).toEqual({ name: 'Alice', model_id: 'gpt-4' })
    expect(result.current.lookups.agents.get('a2')).toEqual({ name: 'Bob', model_id: 'gpt-4' })
    expect(result.current.lookups.agents.size).toBe(2)
  })

  it('builds schema lookup map from schemas', () => {
    const schemas = [makeSchema('s1', 'JSON Output')]
    const { result } = renderHook(() =>
      useCanvasLookups([], [], [], schemas, {}, emptyProtocols, emptyDocDefs, emptyRoster),
    )

    expect(result.current.lookups.outputSchemas.get('s1')).toEqual({ name: 'JSON Output' })
  })

  it('builds step name lookup from steps', () => {
    const steps = [makeStep('step-1', 'single', 'My Step'), makeStep('step-2', 'for_each')]
    const { result } = renderHook(() =>
      useCanvasLookups(steps, [], [], [], {}, emptyProtocols, emptyDocDefs, emptyRoster),
    )

    expect(result.current.lookups.stepNames.get('step-1')).toBe('My Step')
    // Falls back to execution_mode when name is null
    expect(result.current.lookups.stepNames.get('step-2')).toBe('for_each')
  })

  it('builds tools-by-agent lookup', () => {
    const agents = [makeAgent('a1', 'Agent')]
    const toolsByAgent: Record<string, Tool[]> = {
      'a1': [
        { id: 't1', name: 'search', description: '', config: {} } as Tool,
        { id: 't2', name: 'write', description: '', config: {} } as Tool,
      ],
    }
    const { result } = renderHook(() =>
      useCanvasLookups([], [], agents, [], toolsByAgent, emptyProtocols, emptyDocDefs, emptyRoster),
    )

    expect(result.current.lookups.toolsByAgent.get('a1')).toEqual(['search', 'write'])
  })

  it('builds protocol-by-step lookup from step protocols', () => {
    const protocols: Readonly<Record<string, StepProtocolLink>> = {
      'step-1': {
        protocolId: 'proto-1',
        protocolType: 'documenter',
        protocolName: 'Doc Protocol',
        portNames: ['input', 'output'],
      },
    }
    const { result } = renderHook(() =>
      useCanvasLookups([], [], [], [], {}, protocols, emptyDocDefs, emptyRoster),
    )

    const info = result.current.protocolsByStepLookup.get('step-1')
    expect(info).toEqual({
      protocol_type: 'documenter',
      name: 'Doc Protocol',
      portNames: ['input', 'output'],
    })
  })

  it('passes edges through to lookups', () => {
    const edges = [makeEdge('s1', 's2')]
    const { result } = renderHook(() =>
      useCanvasLookups([], edges, [], [], {}, emptyProtocols, emptyDocDefs, emptyRoster),
    )

    expect(result.current.lookups.edges).toBe(edges)
  })

  it('memoizes lookups object across re-renders with same inputs', () => {
    const steps: WorkflowStep[] = []
    const edges: WorkflowStepEdge[] = []
    const agents: Agent[] = []
    const schemas: OutputSchema[] = []
    const tools = {}

    const { result, rerender } = renderHook(
      ({ s, e, a, sc, t }) => useCanvasLookups(s, e, a, sc, t, emptyProtocols, emptyDocDefs, emptyRoster),
      { initialProps: { s: steps, e: edges, a: agents, sc: schemas, t: tools } },
    )

    const first = result.current.lookups
    // Re-render with identical references — useMemo should return same object
    rerender({ s: steps, e: edges, a: agents, sc: schemas, t: tools })
    expect(result.current.lookups).toBe(first)
  })
})
