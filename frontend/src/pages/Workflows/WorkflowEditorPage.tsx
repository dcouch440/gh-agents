import { useEffect } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import Box from '@mui/material/Box'
import CircularProgress from '@mui/material/CircularProgress'
import { useStore, workflowStore, agentStore, outputSchemaStore, protocolStore, workflowExecutionStore } from '@/stores'
import { Board } from '@/components/board'

function WorkflowEditorPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const loading = useStore(workflowStore.store, workflowStore.selectLoading)

  useEffect(() => {
    if (!id) {
      void navigate('/workflows')
      return
    }
    void workflowStore.loadWorkflow(id)
    void agentStore.fetchAll()
    void outputSchemaStore.fetchIfStale()
    void protocolStore.fetchAll()
    // Sequential: workshop hydration runs after latest-run so it gets the final say
    void workflowExecutionStore.hydrateLatestRun(id).then(() =>
      workflowExecutionStore.hydrateWorkshop(id),
    )
    return () => {
      workflowStore.clearActive()
      workflowExecutionStore.reset()
    }
  }, [id, navigate])

  return (
    <Box
      sx={{
        width: '100%',
        height: '100%',
        backgroundColor: (theme) => theme.palette.custom.cavityBg,
        position: 'relative',
      }}
    >
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
  )
}

export { WorkflowEditorPage }
