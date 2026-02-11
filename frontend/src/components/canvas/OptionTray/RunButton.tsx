import { useState, useCallback, useMemo } from 'react'
import MuiButton from '@mui/material/Button'
import CircularProgress from '@mui/material/CircularProgress'
import Tooltip from '@mui/material/Tooltip'
import Fade from '@mui/material/Fade'
import CheckCircleOutline from '@mui/icons-material/CheckCircleOutline'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import ErrorOutline from '@mui/icons-material/ErrorOutline'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore } from '@/stores'
import { api } from '@/api'

type RunStatus = 'idle' | 'running' | 'completed' | 'error'

function RunButton() {
  const theme = useTheme()
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const [runStatus, setRunStatus] = useState<RunStatus>('idle')
  // Single .find() on a small array (<20 steps), not inside a loop — acceptable
  const entryStep = useMemo(() => steps.find((s) => s.execution_mode === 'context') ?? null, [steps])

  const handleRun = useCallback(async () => {
    if (!activeWorkflowId || runStatus === 'running') return
    setRunStatus('running')
    try {
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
  }, [activeWorkflowId, runStatus, entryStep])

  if (!activeWorkflowId) return null

  const runIcon =
    runStatus === 'completed' ? (
      <CheckCircleOutline sx={{ fontSize: 16 }} />
    ) : runStatus === 'error' ? (
      <ErrorOutline sx={{ fontSize: 16 }} />
    ) : (
      <PlayArrowOutlined sx={{ fontSize: 16 }} />
    )

  const runLabel =
    runStatus === 'running' ? 'Running...' : runStatus === 'completed' ? 'Started!' : runStatus === 'error' ? 'Failed' : 'Run'

  const chromeBg = theme.palette.custom.chromeBg
  const statusBg =
    runStatus === 'completed'
      ? theme.palette.success.main
      : runStatus === 'error'
        ? theme.palette.error.main
        : chromeBg

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
            backgroundColor: statusBg,
            boxShadow: 'none',
            '&:hover': {
              backgroundColor: statusBg,
              opacity: 0.9,
              boxShadow: 'none',
            },
            '&.Mui-disabled': {
              backgroundColor: `${statusBg}80`,
              color: 'rgba(255, 255, 255, 0.5)',
              boxShadow: 'none',
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
