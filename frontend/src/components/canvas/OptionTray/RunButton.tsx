import { useState, useCallback } from 'react'
import MuiButton from '@mui/material/Button'
import CircularProgress from '@mui/material/CircularProgress'
import Tooltip from '@mui/material/Tooltip'
import Fade from '@mui/material/Fade'
import Zoom from '@mui/material/Zoom'
import CheckCircleOutline from '@mui/icons-material/CheckCircleOutline'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import ErrorOutline from '@mui/icons-material/ErrorOutline'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore } from '@/stores'
import { api } from '@/api'

type RunStatus = 'idle' | 'running' | 'completed' | 'error'

function RunButton() {
  const theme = useTheme()
  const isDark = theme.palette.mode === 'dark'
  const chromeBg = theme.palette.custom.chromeBg
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const [runStatus, setRunStatus] = useState<RunStatus>('idle')

  const handleRun = useCallback(async () => {
    if (!activeWorkflowId || runStatus === 'running') return
    setRunStatus('running')
    try {
      const entryStep = steps.find((s) => s.execution_mode === 'entry')
      const input = entryStep?.prompt_template.trim()
      const body = input ? { initial_input: input } : undefined
      await api.workflows.run(activeWorkflowId, body)
      setRunStatus('completed')
      setTimeout(() => {
        setRunStatus('idle')
      }, 3000)
    } catch {
      setRunStatus('error')
      setTimeout(() => {
        setRunStatus('idle')
      }, 3000)
    }
  }, [activeWorkflowId, runStatus, steps])

  if (!activeWorkflowId) return null

  const runIcon =
    runStatus === 'completed' ? (
      <Zoom in timeout={200}>
        <CheckCircleOutline sx={{ fontSize: 16 }} />
      </Zoom>
    ) : runStatus === 'error' ? (
      <ErrorOutline sx={{ fontSize: 16 }} />
    ) : (
      <PlayArrowOutlined sx={{ fontSize: 16, transition: 'transform 0.2s ease' }} />
    )

  const runLabel =
    runStatus === 'running' ? 'Running...' : runStatus === 'completed' ? 'Started!' : runStatus === 'error' ? 'Failed' : 'Run'

  const statusColor =
    runStatus === 'completed'
      ? theme.palette.success.main
      : runStatus === 'error'
        ? theme.palette.error.main
        : chromeBg

  const bgBase = runStatus === 'idle' || runStatus === 'running' ? chromeBg : statusColor

  return (
    <Tooltip
      title={
        runStatus === 'running'
          ? 'Workflow is running...'
          : runStatus === 'completed'
            ? 'Execution started successfully'
            : runStatus === 'error'
              ? 'Execution failed to start'
              : 'Run this workflow'
      }
      TransitionComponent={Fade}
      enterDelay={500}
      placement="top"
    >
      <span data-testid="toolbar-run-button">
        <MuiButton
          size="small"
          variant="contained"
          startIcon={runStatus === 'running' ? <CircularProgress size={14} thickness={5} color="inherit" /> : runIcon}
          onClick={() => {
            void handleRun()
          }}
          disabled={runStatus === 'running'}
          sx={{
            fontSize: 13,
            fontWeight: 600,
            textTransform: 'none',
            px: 2.5,
            py: 0.75,
            minWidth: 80,
            color: '#fff',
            background: isDark
              ? `linear-gradient(135deg, ${bgBase} 0%, ${bgBase} 100%)`
              : `linear-gradient(135deg, ${bgBase}dd 0%, ${bgBase} 100%)`,
            boxShadow: `0 2px 8px ${bgBase}33`,
            transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
            '&:hover': {
              background: bgBase,
              boxShadow: `0 4px 14px ${bgBase}4d`,
              transform: 'translateY(-1px)',
            },
            '&:active': {
              transform: 'translateY(0) scale(0.98)',
              boxShadow: `0 2px 8px ${bgBase}33`,
            },
            '&.Mui-disabled': {
              background: `${bgBase}4d`,
              color: 'rgba(255, 255, 255, 0.5)',
            },
          }}
        >
          {runLabel}
        </MuiButton>
      </span>
    </Tooltip>
  )
}

export { RunButton }
