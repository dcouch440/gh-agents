import { useEffect, useMemo } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import AccountTreeOutlined from '@mui/icons-material/AccountTreeOutlined'
import { useStore, workflowStore } from '@/stores'

function WorkflowEditorPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const selector = useMemo(() => id ? workflowStore.selectById(id) : () => undefined, [id])
  const workflow = useStore(workflowStore.store, selector)
  const loading = useStore(workflowStore.store, workflowStore.selectLoading)

  useEffect(() => {
    if (!id) {
      void navigate('/workflows')
      return
    }
    void workflowStore.loadWorkflow(id)
    return () => { workflowStore.clearActive() }
  }, [id, navigate])

  return (
    <Box
      sx={{
        width: '100%',
        height: '100%',
        position: 'relative',
        backgroundImage: 'radial-gradient(circle, rgba(255, 255, 255, 0.06) 1px, transparent 1px)',
        backgroundSize: '20px 20px',
      }}
    >
      {/* Placeholder — React Flow canvas goes here */}
      <Box
        sx={{
          position: 'absolute',
          inset: 0,
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'center',
          alignItems: 'center',
          gap: 1,
          pointerEvents: 'none',
        }}
      >
        <AccountTreeOutlined sx={{ fontSize: 48, color: 'text.disabled' }} />
        <Typography variant="body2" color="text.disabled">
          {loading ? 'Loading workflow...' : (workflow ? workflow.name : 'Workflow canvas')}
        </Typography>
        <Typography variant="caption" color="text.disabled">
          React Flow integration coming soon
        </Typography>
      </Box>
    </Box>
  )
}

export { WorkflowEditorPage }
