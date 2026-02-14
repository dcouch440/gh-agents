import { useEffect, useMemo, useCallback } from 'react'
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import { useStore, agentStore, canvasStore, workflowStore, contextPickerStore } from '@/stores'
import { DESIGN } from '@/constants'
import { Collections } from '@/utils/collections'
import { BrowserPanel } from './BrowserPanel'
import type { Agent } from '@/types/agent'

const matchesQuery = (agent: Agent, query: string) => agent.name.toLowerCase().includes(query.toLowerCase())

const toRow = (agent: Agent) => ({ primary: agent.name, secondary: agent.model_id })

function AgentsBrowserPanel() {
  const agents = useStore(agentStore.store, agentStore.selectAll)
  const loading = useStore(agentStore.store, agentStore.selectLoading)
  const selectedStepIds = useStore(canvasStore.store, canvasStore.selectSelectedStepIds)
  const isPickingActive = useStore(contextPickerStore.store, contextPickerStore.selectActive)

  useEffect(() => {
    void agentStore.fetchAll()
  }, [])

  const firstStepId = useMemo(() => selectedStepIds.values().next().value ?? null, [selectedStepIds])
  const selectedStep = useStore(workflowStore.store, workflowStore.selectStepById(firstStepId))

  const agentsById = useMemo(() => Collections.keyBy(agents, (a) => a.id), [agents])

  const handleAssign = useCallback(
    (agentId: string) => {
      if (!selectedStep) return
      void workflowStore.updateStep(selectedStep.id, { agent_id: agentId })
    },
    [selectedStep],
  )

  const handlePick = useCallback(
    (agentId: string) => {
      const agent = agentsById.get(agentId)
      if (!agent) return
      contextPickerStore.pick({
        kind: 'agent',
        id: agent.id,
        name: agent.name,
        summary: `${agent.model_id} agent`,
        data: agent as unknown as Record<string, unknown>,
      })
    },
    [agentsById],
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
      onPickItem={isPickingActive ? handlePick : null}
    />
  )
}

export { AgentsBrowserPanel }
