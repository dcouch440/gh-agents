import IconButton from '@mui/material/IconButton'
import CircularProgress from '@mui/material/CircularProgress'
import Tooltip from '@mui/material/Tooltip'
import Fade from '@mui/material/Fade'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import CheckCircleOutline from '@mui/icons-material/CheckCircleOutline'
import ErrorOutline from '@mui/icons-material/ErrorOutline'
import { useStore, workflowStore } from '@/stores'
import { useWorkflowRun } from '../../useWorkflowRun'
import { VARIANT_CONFIGS } from '../registry'

const INPUT_ACCENT = VARIANT_CONFIGS.input.color

type InputNodeRunButtonProps = {
  stepId: string
}

function InputNodeRunButton({ stepId }: InputNodeRunButtonProps) {
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const step = useStore(workflowStore.store, workflowStore.selectStepById(stepId))
  const promptTemplate = step?.prompt_template ?? ''

  const { status, handleRun, tooltipText } = useWorkflowRun(promptTemplate)

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

  const bgColor =
    status === 'completed'
      ? '#22c55e'
      : status === 'error'
        ? '#ef4444'
        : `${INPUT_ACCENT}30`

  return (
    <Tooltip title={tooltipText} TransitionComponent={Fade} enterDelay={300} placement="top">
      <IconButton
        size="small"
        onClick={handleRun}
        disabled={status === 'running'}
        sx={{
          width: 28,
          height: 28,
          borderRadius: '6px',
          backgroundColor: bgColor,
          color: status === 'completed' || status === 'error' ? '#fff' : INPUT_ACCENT,
          '&:hover': { backgroundColor: `${INPUT_ACCENT}50` },
          '&.Mui-disabled': { backgroundColor: `${INPUT_ACCENT}20`, color: `${INPUT_ACCENT}80` },
          transition: 'background-color 150ms ease, color 150ms ease',
        }}
      >
        {icon}
      </IconButton>
    </Tooltip>
  )
}

export { InputNodeRunButton }
