import { useMemo } from 'react'
import MuiButton from '@mui/material/Button'
import CircularProgress from '@mui/material/CircularProgress'
import Tooltip from '@mui/material/Tooltip'
import Fade from '@mui/material/Fade'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import ErrorOutline from '@mui/icons-material/ErrorOutline'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore } from '@/stores'
import { useWorkflowRun } from '../useWorkflowRun'
import { TRAY_BUTTON_CONTAINED_SX } from './constants'

function RunButton() {
  const theme = useTheme()
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)

  // Prefer input step over context step as entry point — single .find() on small array
  const entryStep = useMemo(() => {
    const inputStep = steps.find((s) => s.execution_mode === 'input')
    if (inputStep) return inputStep
    return steps.find((s) => s.execution_mode === 'context') ?? null
  }, [steps])

  const { status, handleRun, tooltipText } = useWorkflowRun(entryStep?.prompt_template ?? '')

  if (!activeWorkflowId) return null

  const runIcon =
    status === 'error' ? (
      <ErrorOutline sx={{ fontSize: 16 }} />
    ) : (
      <PlayArrowOutlined sx={{ fontSize: 16 }} />
    )

  const runLabel =
    status === 'running' ? 'Running...' : status === 'error' ? 'Failed' : 'Run'

  const chromeBg = theme.palette.custom.chromeBg
  const statusBg = status === 'error' ? theme.palette.error.main : chromeBg

  return (
    <Tooltip
      title={tooltipText}
      TransitionComponent={Fade}
      enterDelay={500}
      placement="top"
    >
      <span data-testid="toolbar-run-button">
        <MuiButton
          size="small"
          variant="contained"
          startIcon={status === 'running' ? <CircularProgress size={14} thickness={5} color="inherit" /> : runIcon}
          onClick={handleRun}
          disabled={status === 'running'}
          sx={{
            ...TRAY_BUTTON_CONTAINED_SX,
            minWidth: 80,
            backgroundColor: statusBg,
            '&:hover': { backgroundColor: statusBg, opacity: 0.9, boxShadow: 'none' },
            '&.Mui-disabled': { backgroundColor: `${statusBg}80`, color: 'rgba(255, 255, 255, 0.5)', boxShadow: 'none' },
          }}
        >
          {runLabel}
        </MuiButton>
      </span>
    </Tooltip>
  )
}

export { RunButton }
