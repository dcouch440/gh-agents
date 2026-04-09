import { useEffect } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import Box from '@mui/material/Box'
import CircularProgress from '@mui/material/CircularProgress'
import { useStore, workflowStore, agentStore, outputSchemaStore, protocolStore, workflowExecutionStore, sidebarStore } from '@/stores'
import { Board } from '@/components/board'
import { WorkflowSidebar } from '@/components/sidebar'
import { useWorkflowAgentChat } from '@/hooks/useWorkflowAgentChat'

function WorkflowEditorPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const loading = useStore(workflowStore.store, workflowStore.selectLoading)
  const { messages, sendMessage, streaming, cancelChat, submitPanel } = useWorkflowAgentChat(id ?? null)
  useEffect(() => {
    if (!id) {
      void navigate('/workflows')
      return
    }
    const loadWorkflowWithRosters = workflowStore.loadWorkflow(id).then(() => {
      // Fetch roster agents for all steps so the sidebar tree can display them
      const steps = workflowStore.selectSteps(workflowStore.store.getState())
      return Promise.all(steps.map((step) => workflowStore.fetchRoster(step.id)))
    })
    void agentStore.fetchAll()
    void outputSchemaStore.fetchIfStale()
    void protocolStore.fetchAll()
    // Sequential: workshop hydration runs after latest-run so it gets the final say
    const hydrateRun = workflowExecutionStore.hydrateLatestRun(id)
      .then(() => workflowExecutionStore.hydrateWorkshop(id))
    // After both workflow+rosters and run data are loaded, hydrate agent sources
    void Promise.all([loadWorkflowWithRosters, hydrateRun]).then(() => {
      const rosterByStep = workflowStore.selectRosterByStep(workflowStore.store.getState())
      workflowExecutionStore.hydrateAgentSources(rosterByStep)
    })
    return () => {
      workflowStore.clearActive()
      workflowExecutionStore.reset()
      sidebarStore.reset()
    }
  }, [id, navigate])

  return (
    <Box
      sx={{
        display: 'flex',
        width: '100%',
        height: '100%',
        backgroundColor: (theme) => theme.palette.custom.cavityBg,
      }}
    >
      {/* Canvas */}
      <Box sx={{ flex: 1, minWidth: 0, position: 'relative' }}>
        {id && <Board workflowId={id} />}
        {loading && (
          <Box
            sx={{
              position: 'absolute',
              inset: 0,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              zIndex: 5,
              backgroundColor: (theme) => theme.palette.custom.cavityBg,
            }}
          >
            <CircularProgress size={32} />
          </Box>
        )}
      </Box>

      {/* Sidebar (includes Chat tab) */}
      {id && (
        <WorkflowSidebar
          messages={messages}
          onSend={sendMessage}
          onCancel={cancelChat}
          streaming={streaming}
          onPanelSubmit={submitPanel}
        />
      )}
    </Box>
  )
}

export { WorkflowEditorPage }
