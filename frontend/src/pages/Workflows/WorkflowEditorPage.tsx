import { useEffect } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import Box from '@mui/material/Box'
import CircularProgress from '@mui/material/CircularProgress'
import { useStore, workflowStore, agentStore, outputSchemaStore, protocolStore, workflowExecutionStore } from '@/stores'
import { WorkflowCanvas } from '@/components/canvas'

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
    void workflowExecutionStore.hydrateLatestRun(id)
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
      <WorkflowCanvas />
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
