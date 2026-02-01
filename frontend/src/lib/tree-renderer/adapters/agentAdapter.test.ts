import { describe, it, expect } from 'vitest'
import { agentHierarchyToTree } from './agentAdapter'
import type { Agent } from '@/types'

const createMockAgent = (overrides: Partial<Agent>): Agent => ({
  id: 'agent-001',
  tier: 'worker',
  persona_name: 'TestBot',
  persona_prompt: 'You are a test agent.',
  persona_style: 'concise',
  model_provider: 'anthropic',
  model_id: 'claude-sonnet-4-20250514',
  model_max_tokens: 8192,
  model_temperature: 0.7,
  status: 'idle',
  ...overrides,
})

describe('agentHierarchyToTree', () => {
  it('converts empty agents array to tree with tier roots', () => {
    const result = agentHierarchyToTree([])

    expect(result.rootIds).toEqual([
      'tier-orchestrator',
      'tier-worker',
      'tier-utility',
    ])
    expect(result.nodes['tier-orchestrator']).toMatchObject({
      id: 'tier-orchestrator',
      label: 'ORCHESTRATOR',
      status: 'completed',
      children: [],
    })
    expect(result.nodes['tier-worker']).toMatchObject({
      id: 'tier-worker',
      label: 'WORKER',
      status: 'completed',
      children: [],
    })
    expect(result.nodes['tier-utility']).toMatchObject({
      id: 'tier-utility',
      label: 'UTILITY',
      status: 'completed',
      children: [],
    })
    expect(result.edges).toEqual([])
  })

  it('creates agent nodes grouped by tier', () => {
    const agents = [
      createMockAgent({ id: 'agent-1', tier: 'orchestrator', persona_name: 'Leader' }),
      createMockAgent({ id: 'agent-2', tier: 'worker', persona_name: 'Worker1' }),
      createMockAgent({ id: 'agent-3', tier: 'worker', persona_name: 'Worker2' }),
      createMockAgent({ id: 'agent-4', tier: 'utility', persona_name: 'Helper' }),
    ]
    const result = agentHierarchyToTree(agents)

    expect(result.nodes['tier-orchestrator']?.children).toEqual(['agent-1'])
    expect(result.nodes['tier-worker']?.children).toEqual(['agent-2', 'agent-3'])
    expect(result.nodes['tier-utility']?.children).toEqual(['agent-4'])
  })

  it('creates edges from tier to agents', () => {
    const agents = [
      createMockAgent({ id: 'agent-1', tier: 'worker', persona_name: 'Worker' }),
    ]
    const result = agentHierarchyToTree(agents)

    expect(result.edges).toEqual([
      {
        sourceId: 'tier-worker',
        targetId: 'agent-1',
        label: null,
        variant: 'normal',
      },
    ])
  })

  it('converts agent statuses to node statuses', () => {
    const agents = [
      createMockAgent({ id: 'agent-1', status: 'idle' }),
      createMockAgent({ id: 'agent-2', status: 'working' }),
      createMockAgent({ id: 'agent-3', status: 'waiting_for_context' }),
      createMockAgent({ id: 'agent-4', status: 'waiting_for_approval' }),
    ]
    const result = agentHierarchyToTree(agents)

    expect(result.nodes['agent-1']?.status).toBe('completed')
    expect(result.nodes['agent-2']?.status).toBe('running')
    expect(result.nodes['agent-3']?.status).toBe('waiting')
    expect(result.nodes['agent-4']?.status).toBe('waiting')
  })

  it('sets tier status to running if any agent is working', () => {
    const agents = [
      createMockAgent({ id: 'agent-1', tier: 'worker', status: 'idle' }),
      createMockAgent({ id: 'agent-2', tier: 'worker', status: 'working' }),
    ]
    const result = agentHierarchyToTree(agents)

    expect(result.nodes['tier-worker']?.status).toBe('running')
  })

  it('sets tier status to completed if all agents are idle', () => {
    const agents = [
      createMockAgent({ id: 'agent-1', tier: 'worker', status: 'idle' }),
      createMockAgent({ id: 'agent-2', tier: 'worker', status: 'idle' }),
    ]
    const result = agentHierarchyToTree(agents)

    expect(result.nodes['tier-worker']?.status).toBe('completed')
  })

  it('preserves agent metadata', () => {
    const agent = createMockAgent({
      id: 'agent-1',
      tier: 'worker',
      persona_name: 'TestBot',
      model_id: 'claude-sonnet-4-20250514',
      persona_style: 'verbose',
    })
    const result = agentHierarchyToTree([agent])

    expect(result.nodes['agent-1']?.label).toBe('TestBot')
    expect(result.nodes['agent-1']?.metadata).toEqual({
      tier: 'worker',
      modelId: 'claude-sonnet-4-20250514',
      personaStyle: 'verbose',
    })
  })

  it('creates all tier nodes even if some have no agents', () => {
    const agents = [
      createMockAgent({ id: 'agent-1', tier: 'worker' }),
    ]
    const result = agentHierarchyToTree(agents)

    expect(result.rootIds).toEqual([
      'tier-orchestrator',
      'tier-worker',
      'tier-utility',
    ])
    expect(result.nodes['tier-orchestrator']?.children).toEqual([])
    expect(result.nodes['tier-worker']?.children).toEqual(['agent-1'])
    expect(result.nodes['tier-utility']?.children).toEqual([])
  })

  it('handles multiple agents in each tier', () => {
    const agents = [
      createMockAgent({ id: 'agent-1', tier: 'orchestrator' }),
      createMockAgent({ id: 'agent-2', tier: 'orchestrator' }),
      createMockAgent({ id: 'agent-3', tier: 'worker' }),
      createMockAgent({ id: 'agent-4', tier: 'worker' }),
      createMockAgent({ id: 'agent-5', tier: 'utility' }),
    ]
    const result = agentHierarchyToTree(agents)

    expect(result.nodes['tier-orchestrator']?.children).toHaveLength(2)
    expect(result.nodes['tier-worker']?.children).toHaveLength(2)
    expect(result.nodes['tier-utility']?.children).toHaveLength(1)
    expect(result.edges).toHaveLength(5)
  })
})
