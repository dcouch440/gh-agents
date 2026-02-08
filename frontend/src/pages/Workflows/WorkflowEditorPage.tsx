import { useEffect } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import Box from '@mui/material/Box'
import { useStore, workflowStore } from '@/stores'
import { WorkflowCanvas } from '@/components/canvas'
import { LoadingSpinner } from '@/components/primitives'
import { LAYOUT_COLORS } from '@/constants'

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
    return () => { workflowStore.clearActive() }
  }, [id, navigate])

  if (loading) {
    return (
      <Box
        sx={{
          width: '100%',
          height: '100%',
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
        }}
      >
        <LoadingSpinner label="Loading workflow..." />
      </Box>
    )
  }

  return (
    <Box
      sx={{
        width: '100%',
        height: '100%',
        backgroundColor: LAYOUT_COLORS.CAVITY_BG,
      }}
    >
      <WorkflowCanvas />
    </Box>
  )
}

export { WorkflowEditorPage }
