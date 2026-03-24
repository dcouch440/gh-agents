import { useEffect, useRef } from 'react'
import { agentStore, workflowStore, protocolStore } from '@/stores'
import type { Agent } from '@/types/agent'
import type { WorkflowStep } from '@/types/workflow'

/**
 * Manages lazy data-fetching for the canvas:
 * - Tools for each agent (once per agent)
 * - Document defs + roster for workforce steps (once per step)
 * - Protocol catalog (once on mount)
 */
const useCanvasFetch = (
  agents: readonly Agent[],
  steps: readonly WorkflowStep[],
): void => {
  const fetchedToolAgentIds = useRef(new Set<string>())
  const fetchedRosterStepIds = useRef(new Set<string>())

  // Fetch tools for agents not yet fetched
  useEffect(() => {
    for (let i = 0; i < agents.length; i++) {
      const agent = agents[i]!
      if (!fetchedToolAgentIds.current.has(agent.id)) {
        fetchedToolAgentIds.current.add(agent.id)
        void agentStore.fetchTools(agent.id)
      }
    }
  }, [agents])

  // Fetch rosters for workforce steps
  useEffect(() => {
    for (let i = 0; i < steps.length; i++) {
      const step = steps[i]!
      if (step.execution_mode === 'workforce') {
        if (!fetchedRosterStepIds.current.has(step.id)) {
          fetchedRosterStepIds.current.add(step.id)
          void workflowStore.fetchRoster(step.id)
        }
      }
    }
  }, [steps])

  // Fetch protocol catalog once (notes are loaded by loadWorkflow)
  useEffect(() => {
    void protocolStore.fetchAll()
    void protocolStore.fetchTypes()
  }, [])
}

export { useCanvasFetch }
