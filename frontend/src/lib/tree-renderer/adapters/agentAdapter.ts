import type { Agent } from '@/types'
import type { TreeData, TreeNode, TreeEdgeData, NodeStatus } from '../types'

type AgentMeta = {
  tier: string
  modelId: string
  personaStyle: string
}

const agentStatusToNodeStatus = (status: string): NodeStatus => {
  if (status === 'working') return 'running'
  if (status === 'waiting_for_context' || status === 'waiting_for_approval') return 'waiting'
  return 'completed' // idle = completed (available)
}

const agentHierarchyToTree = (agents: Agent[]): TreeData<AgentMeta> => {
  const nodes: Record<string, TreeNode<AgentMeta>> = {}
  const edges: TreeEdgeData[] = []

  // Create tier root nodes
  const tiers = ['orchestrator', 'worker', 'utility'] as const
  const tierIds = tiers.map((t) => `tier-${t}`)

  for (const tier of tiers) {
    const tierId = `tier-${tier}`
    const tierAgents = agents.filter((a) => a.tier === tier)

    nodes[tierId] = {
      id: tierId,
      label: tier.toUpperCase(),
      status: tierAgents.some((a) => a.status === 'working') ? 'running' : 'completed',
      children: tierAgents.map((a) => a.id),
      metadata: { tier, modelId: '', personaStyle: '' },
    }

    for (const agent of tierAgents) {
      nodes[agent.id] = {
        id: agent.id,
        label: agent.persona_name,
        status: agentStatusToNodeStatus(agent.status),
        children: [],
        metadata: {
          tier: agent.tier,
          modelId: agent.model_id,
          personaStyle: agent.persona_style,
        },
      }

      edges.push({
        sourceId: tierId,
        targetId: agent.id,
        label: null,
        variant: 'normal',
      })
    }
  }

  return { nodes, rootIds: tierIds, edges }
}

export { agentHierarchyToTree }
export type { AgentMeta }
