import { useState, useCallback } from 'react'
import IconButton from '@mui/material/IconButton'
import CircularProgress from '@mui/material/CircularProgress'
import Tooltip from '@mui/material/Tooltip'
import Fade from '@mui/material/Fade'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import CheckCircleOutline from '@mui/icons-material/CheckCircleOutline'
import ErrorOutline from '@mui/icons-material/ErrorOutline'
import { useStore, workflowStore } from '@/stores'
import { api } from '@/api'
import { INPUT_NODE } from './constants'

type RunStatus = 'idle' | 'running' | 'completed' | 'error'

type InputNodeRunButtonProps = {
  stepId: string
}

function InputNodeRunButton({ stepId }: InputNodeRunButtonProps) {
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const step = useStore(workflowStore.store, workflowStore.selectStepById(stepId))
  const promptTemplate = step?.prompt_template ?? ''
  const [status, setStatus] = useState<RunStatus>('idle')

  const handleRun = useCallback(async () => {
    if (!activeWorkflowId || status === 'running') return
    setStatus('running')
    try {
      const input = promptTemplate.trim()
      const body = input ? { initial_input: input } : undefined
      await api.workflows.run(activeWorkflowId, body)
      setStatus('completed')
      setTimeout(() => {
        setStatus('idle')
      }, 3000)
    } catch {
      setStatus('error')
      setTimeout(() => {
        setStatus('idle')
      }, 3000)
    }
  }, [activeWorkflowId, status, promptTemplate])

  if (!activeWorkflowId) return null

  const icon =
    status === 'running' ? (
      <CircularProgress size={14} thickness={5} color="inherit" />
    ) : status === 'completed' ? (
      <CheckCircleOutline sx={{ fontSize: 16 }} />
    ) : status === 'error' ? (
      <ErrorOutline sx={{ fontSize: 16 }} />
    ) : (
      <PlayArrowOutlined sx={{ fontSize: 16 }} />
    )

  const tooltipText =
    status === 'running'
      ? 'Workflow is running...'
      : status === 'completed'
        ? 'Execution started successfully'
        : status === 'error'
          ? 'Execution failed to start'
          : 'Run workflow'

  const bgColor =
    status === 'completed'
      ? '#22c55e'
      : status === 'error'
        ? '#ef4444'
        : `${INPUT_NODE.ACCENT_COLOR}30`

  return (
    <Tooltip title={tooltipText} TransitionComponent={Fade} enterDelay={300} placement="top">
      <IconButton
        size="small"
        onClick={() => {
          void handleRun()
        }}
        disabled={status === 'running'}
        sx={{
          width: 28,
          height: 28,
          borderRadius: '6px',
          backgroundColor: bgColor,
          color: status === 'completed' || status === 'error' ? '#fff' : INPUT_NODE.ACCENT_COLOR,
          '&:hover': { backgroundColor: `${INPUT_NODE.ACCENT_COLOR}50` },
          '&.Mui-disabled': { backgroundColor: `${INPUT_NODE.ACCENT_COLOR}20`, color: `${INPUT_NODE.ACCENT_COLOR}80` },
          transition: 'background-color 150ms ease, color 150ms ease',
        }}
      >
        {icon}
      </IconButton>
    </Tooltip>
  )
}

export { InputNodeRunButton }
