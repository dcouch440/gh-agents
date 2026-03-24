import { describe, it, expect } from 'vitest'
import { computeProtocolGroups, isWorkforceStep } from './protocolGroups'
import type { ProtocolStepInfo } from './types'
import type { WorkflowStep } from '@/types/workflow'

const baseStep: WorkflowStep = {
  id: 'step-001',
  workflow_id: 'wf-001',
  name: 'Step',
  agent_id: 'agent-001',
  execution_mode: 'single',
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
  system_prompt_suffix: null,
}

describe('isWorkforceStep', () => {
  it('returns true when execution_mode is workforce', () => {
    const step = { id: 'step-1', execution_mode: 'workforce' as const }
    expect(isWorkforceStep(step, new Map())).toBe(true)
  })

  it('returns true when protocol type is workforce', () => {
    const step = { id: 'step-1', execution_mode: 'single' }
    const protocols = new Map<string, ProtocolStepInfo>([
      ['step-1', { protocol_type: 'workforce', name: 'Team', portNames: [] }],
    ])
    expect(isWorkforceStep(step, protocols)).toBe(true)
  })

  it('returns false for non-workforce steps', () => {
    const step = { id: 'step-1', execution_mode: 'single' }
    expect(isWorkforceStep(step, new Map())).toBe(false)
  })

  it('returns false for a different protocol type', () => {
    const step = { id: 'step-1', execution_mode: 'single' }
    const protocols = new Map<string, ProtocolStepInfo>([
      ['step-1', { protocol_type: 'decomp', name: 'Decomp', portNames: [] }],
    ])
    expect(isWorkforceStep(step, protocols)).toBe(false)
  })
})

describe('computeProtocolGroups', () => {
  it('returns empty map when no protocol steps exist', () => {
    const steps = [
      { ...baseStep, id: 'a' },
      { ...baseStep, id: 'b' },
    ]
    const edges = [{ from_step_id: 'a', to_step_id: 'b' }]
    const result = computeProtocolGroups(steps, edges, new Map())
    expect(result.size).toBe(0)
  })

  it('assigns connected non-protocol nodes to the protocol group', () => {
    const steps = [
      { ...baseStep, id: 'proto', execution_mode: 'workforce' as const },
      { ...baseStep, id: 'worker-1' },
      { ...baseStep, id: 'worker-2' },
    ]
    const edges = [
      { from_step_id: 'proto', to_step_id: 'worker-1' },
      { from_step_id: 'worker-1', to_step_id: 'worker-2' },
    ]
    const result = computeProtocolGroups(steps, edges, new Map())

    expect(result.has('worker-1')).toBe(true)
    expect(result.has('worker-2')).toBe(true)
    expect(result.get('worker-1')!.protocolStepId).toBe('proto')
    expect(result.get('worker-2')!.protocolStepId).toBe('proto')
  })

  it('does not include protocol steps themselves in the result', () => {
    const steps = [
      { ...baseStep, id: 'proto', execution_mode: 'workforce' as const },
      { ...baseStep, id: 'worker' },
    ]
    const edges = [{ from_step_id: 'proto', to_step_id: 'worker' }]
    const result = computeProtocolGroups(steps, edges, new Map())

    expect(result.has('proto')).toBe(false)
    expect(result.has('worker')).toBe(true)
  })

  it('handles isolated protocol steps with no edges', () => {
    const steps = [
      { ...baseStep, id: 'proto', execution_mode: 'workforce' as const },
    ]
    const result = computeProtocolGroups(steps, [], new Map())
    expect(result.size).toBe(0)
  })

  it('uses protocol color from PROTOCOL_TYPE_COLORS', () => {
    const steps = [
      { ...baseStep, id: 'proto', execution_mode: 'workforce' as const },
      { ...baseStep, id: 'worker' },
    ]
    const edges = [{ from_step_id: 'proto', to_step_id: 'worker' }]
    const result = computeProtocolGroups(steps, edges, new Map())

    expect(result.get('worker')!.protocolColor).toBe('#3b82f6')
  })

  it('handles protocolsByStep map for protocol detection', () => {
    const steps = [
      { ...baseStep, id: 'proto' },
      { ...baseStep, id: 'worker' },
    ]
    const edges = [{ from_step_id: 'proto', to_step_id: 'worker' }]
    const protocols = new Map<string, ProtocolStepInfo>([
      ['proto', { protocol_type: 'decomp', name: 'Decomp', portNames: [] }],
    ])
    const result = computeProtocolGroups(steps, edges, protocols)

    expect(result.has('worker')).toBe(true)
    expect(result.get('worker')!.protocolStepId).toBe('proto')
    expect(result.get('worker')!.protocolColor).toBe('#3b82f6')
  })

  it('assigns each group independently when protocols are disconnected', () => {
    const steps = [
      { ...baseStep, id: 'proto-a', execution_mode: 'workforce' as const },
      { ...baseStep, id: 'worker-a' },
      { ...baseStep, id: 'proto-b', execution_mode: 'workforce' as const },
      { ...baseStep, id: 'worker-b' },
    ]
    const edges = [
      { from_step_id: 'proto-a', to_step_id: 'worker-a' },
      { from_step_id: 'proto-b', to_step_id: 'worker-b' },
    ]
    const result = computeProtocolGroups(steps, edges, new Map())

    expect(result.get('worker-a')!.protocolStepId).toBe('proto-a')
    expect(result.get('worker-b')!.protocolStepId).toBe('proto-b')
  })
})
