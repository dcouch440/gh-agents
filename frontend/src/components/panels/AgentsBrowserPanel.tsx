import { useEffect, useMemo, useCallback } from 'react'
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import { useStore, agentStore, canvasStore, workflowStore } from '@/stores'
import { DESIGN } from '@/constants'
import { BrowserPanel } from './BrowserPanel'
import type { Agent } from '@/types/agent'

const matchesQuery = (agent: Agent, query: string) => agent.name.toLowerCase().includes(query.toLowerCase())

const toRow = (agent: Agent) => ({ primary: agent.name, secondary: agent.model_id })

function AgentsBrowserPanel() {
  const agents = useStore(agentStore.store, agentStore.selectAll)
  const loading = useStore(agentStore.store, agentStore.selectLoading)
  const selectedStepIds = useStore(canvasStore.store, canvasStore.selectSelectedStepIds)

  useEffect(() => {
    void agentStore.fetchAll()
  }, [])

  const firstStepId = useMemo(() => selectedStepIds.values().next().value ?? null, [selectedStepIds])
  const selectedStep = useStore(workflowStore.store, workflowStore.selectStepById(firstStepId))

  const handleAssign = useCallback(
    (agentId: string) => {
      if (!selectedStep) return
      void workflowStore.updateStep(selectedStep.id, { agent_id: agentId })
    },
    [selectedStep],
  )

  const isHighlighted = useCallback((agent: Agent) => agent.id === selectedStep?.agent_id, [selectedStep])

  return (
    <BrowserPanel
      items={agents}
      loading={loading}
      searchPlaceholder="Search agents..."
      emptyIcon={<SmartToyOutlined />}
      emptyLabel="agents"
      barColor={DESIGN.PORT_STRING}
      toRow={toRow}
      matchesQuery={matchesQuery}
      isHighlighted={isHighlighted}
      onItemClick={selectedStep ? handleAssign : null}
    />
  )
}

export { AgentsBrowserPanel }
