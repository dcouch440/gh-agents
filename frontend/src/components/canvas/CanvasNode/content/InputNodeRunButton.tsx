import IconButton from '@mui/material/IconButton'
import Tooltip from '@mui/material/Tooltip'
import Fade from '@mui/material/Fade'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import StopOutlined from '@mui/icons-material/StopOutlined'
import ErrorOutline from '@mui/icons-material/ErrorOutline'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore } from '@/stores'
import { useWorkflowRun } from '../../useWorkflowRun'

type InputNodeRunButtonProps = {
  stepId: string
}

function InputNodeRunButton({ stepId }: InputNodeRunButtonProps) {
  const theme = useTheme()
  const inputAccent = theme.palette.nodePalette.input
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const step = useStore(workflowStore.store, workflowStore.selectStepById(stepId))
  const promptTemplate = step?.prompt_template ?? ''

  const { status, handleRun, handleCancel, tooltipText } = useWorkflowRun(promptTemplate)

  if (!activeWorkflowId) return null

  const icon =
    status === 'running' ? (
      <StopOutlined sx={{ fontSize: 16 }} />
    ) : status === 'error' ? (
      <ErrorOutline sx={{ fontSize: 16 }} />
    ) : (
      <PlayArrowOutlined sx={{ fontSize: 16 }} />
    )

  const bgColor = status === 'error' ? '#ef4444' : `${inputAccent}30`

  return (
    <Tooltip title={tooltipText} TransitionComponent={Fade} enterDelay={300} placement="top">
      <IconButton
        size="small"
        onClick={status === 'running' ? handleCancel : handleRun}
        sx={{
          width: 28,
          height: 28,
          borderRadius: '6px',
          backgroundColor: bgColor,
          color: status === 'error' ? '#fff' : inputAccent,
          '&:hover': { backgroundColor: `${inputAccent}50` },
          '&.Mui-disabled': { backgroundColor: `${inputAccent}20`, color: `${inputAccent}80` },
          transition: 'background-color 150ms ease, color 150ms ease',
        }}
      >
        {icon}
      </IconButton>
    </Tooltip>
  )
}

export { InputNodeRunButton }
