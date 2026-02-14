import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useCanvasFetch } from './useCanvasFetch'
import type { Agent } from '@/types/agent'
import type { WorkflowStep } from '@/types/workflow'

const mocks = vi.hoisted(() => ({
  fetchTools: vi.fn<(id: string) => void>(),
  fetchDocumentDefs: vi.fn<(id: string) => void>(),
  fetchRoster: vi.fn<(id: string) => void>(),
  fetchAllProtocols: vi.fn<() => void>(),
  fetchProtocolTypes: vi.fn<() => void>(),
}))

vi.mock('@/stores', () => ({
  agentStore: { fetchTools: mocks.fetchTools },
  workflowStore: {
    fetchDocumentDefs: mocks.fetchDocumentDefs,
    fetchRoster: mocks.fetchRoster,
  },
  protocolStore: {
    fetchAll: mocks.fetchAllProtocols,
    fetchTypes: mocks.fetchProtocolTypes,
  },
}))

const makeAgent = (id: string): Agent => ({
  id,
  name: `Agent ${id}`,
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

const makeStep = (id: string, mode: string): WorkflowStep => ({
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
  name: null,
  room_id: null,
  system_prompt_suffix: null,
  description: '',
})

beforeEach(() => {
  vi.clearAllMocks()
})

describe('useCanvasFetch', () => {
  it('fetches tools for each agent once', () => {
    const agents = [makeAgent('a1'), makeAgent('a2')]
    const { rerender } = renderHook(
      ({ a, s }) => useCanvasFetch(a, s),
      { initialProps: { a: agents, s: [] as WorkflowStep[] } },
    )

    expect(mocks.fetchTools).toHaveBeenCalledTimes(2)
    expect(mocks.fetchTools).toHaveBeenCalledWith('a1')
    expect(mocks.fetchTools).toHaveBeenCalledWith('a2')

    // Re-render with same agents — should not re-fetch
    rerender({ a: agents, s: [] })
    expect(mocks.fetchTools).toHaveBeenCalledTimes(2)
  })

  it('fetches document defs for documenter steps once', () => {
    const steps = [makeStep('s1', 'documenter')]
    renderHook(() => useCanvasFetch([], steps))

    expect(mocks.fetchDocumentDefs).toHaveBeenCalledTimes(1)
    expect(mocks.fetchDocumentDefs).toHaveBeenCalledWith('s1')
  })

  it('fetches roster for task_force steps once', () => {
    const steps = [makeStep('s1', 'task_force')]
    renderHook(() => useCanvasFetch([], steps))

    expect(mocks.fetchRoster).toHaveBeenCalledTimes(1)
    expect(mocks.fetchRoster).toHaveBeenCalledWith('s1')
  })

  it('does not fetch for non-special step types', () => {
    const steps = [makeStep('s1', 'single')]
    renderHook(() => useCanvasFetch([], steps))

    expect(mocks.fetchDocumentDefs).not.toHaveBeenCalled()
    expect(mocks.fetchRoster).not.toHaveBeenCalled()
  })

  it('fetches protocol catalog on mount', () => {
    renderHook(() => useCanvasFetch([], []))

    expect(mocks.fetchAllProtocols).toHaveBeenCalledTimes(1)
    expect(mocks.fetchProtocolTypes).toHaveBeenCalledTimes(1)
  })

  it('deduplicates tool fetches when new agents are added', () => {
    const agents1 = [makeAgent('a1')]
    const agents2 = [makeAgent('a1'), makeAgent('a2')]

    const { rerender } = renderHook(
      ({ a }) => useCanvasFetch(a, []),
      { initialProps: { a: agents1 } },
    )

    expect(mocks.fetchTools).toHaveBeenCalledTimes(1)

    rerender({ a: agents2 })
    expect(mocks.fetchTools).toHaveBeenCalledTimes(2)
    expect(mocks.fetchTools).toHaveBeenCalledWith('a2')
  })
})
