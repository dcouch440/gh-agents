import { useState, useEffect, useMemo, useCallback } from 'react'
import Box from '@mui/material/Box'
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import { SearchInput, AccentBarRow, EmptyState, LoadingSpinner } from '@/components/primitives'
import { useStore, agentStore, canvasStore, workflowStore } from '@/stores'
import { DESIGN } from '@/constants'

function AgentsBrowserPanel() {
  const [query, setQuery] = useState('')
  const agents = useStore(agentStore.store, agentStore.selectAll)
  const loading = useStore(agentStore.store, agentStore.selectLoading)
  const selectedStepIds = useStore(canvasStore.store, canvasStore.selectSelectedStepIds)
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)

  useEffect(() => {
    void agentStore.fetchAll()
  }, [])

  const selectedStep = useMemo(() => {
    const firstId = selectedStepIds.values().next().value
    if (!firstId) return null
    return steps.find((s) => s.id === firstId) ?? null
  }, [selectedStepIds, steps])

  const filtered = useMemo(
    () => agents.filter((a) => a.name.toLowerCase().includes(query.toLowerCase())),
    [agents, query],
  )

  const handleAssign = useCallback(
    (agentId: string) => {
      if (!selectedStep) return
      void workflowStore.updateStep(selectedStep.id, { agent_id: agentId })
    },
    [selectedStep],
  )

  return (
    <Box>
      <Box sx={{ px: 1.5, py: 1 }}>
        <SearchInput value={query} onChange={setQuery} placeholder="Search agents..." />
      </Box>

      {loading ? <LoadingSpinner label="Loading agents..." /> : null}

      {!loading && filtered.length === 0 ? (
        <EmptyState
          icon={<SmartToyOutlined />}
          message={query ? `No agents matching "${query}"` : 'No agents found'}
        />
      ) : null}

      {filtered.map((agent) => (
        <AccentBarRow
          key={agent.id}
          barColor={DESIGN.PORT_STRING}
          primary={agent.name}
          secondary={agent.model_id}
          highlight={agent.id === selectedStep?.agent_id}
          onClick={selectedStep ? () => { handleAssign(agent.id) } : null}
        />
      ))}
    </Box>
  )
}

export { AgentsBrowserPanel }
