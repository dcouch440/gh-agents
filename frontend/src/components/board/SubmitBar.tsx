import type { ReactNode } from 'react'
import Paper from '@mui/material/Paper'
import IconButton from '@mui/material/IconButton'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import CheckCircleOutline from '@mui/icons-material/CheckCircleOutline'
import ErrorOutline from '@mui/icons-material/ErrorOutline'
import AutoAwesomeIcon from '@mui/icons-material/AutoAwesome'
import AccountTreeIcon from '@mui/icons-material/AccountTree'
import { GradientButton } from '@/components/primitives'
import type { RunStatus } from '@/components/canvas/useWorkflowRun'

type SubmitBarProps = {
  readonly onGenerate: () => void
  readonly isGenerating: boolean
  readonly onRun: () => void
  readonly runStatus: RunStatus
  readonly showDebug: boolean
  readonly onToggleDebug: () => void
}

const RUN_ICON: Record<RunStatus, ReactNode> = {
  idle: <PlayArrowOutlined sx={{ fontSize: 18 }} />,
  running: null,
  completed: <CheckCircleOutline sx={{ fontSize: 18 }} />,
  error: <ErrorOutline sx={{ fontSize: 18 }} />,
}

const RUN_LABEL: Record<RunStatus, string> = {
  idle: 'Run',
  running: 'Running…',
  completed: 'Started!',
  error: 'Failed',
}

function SubmitBar({ onGenerate, isGenerating, onRun, runStatus, showDebug, onToggleDebug }: SubmitBarProps) {
  return (
    <Paper
      elevation={4}
      sx={{
        position: 'absolute',
        bottom: 24,
        left: '50%',
        transform: 'translateX(-50%)',
        display: 'flex',
        alignItems: 'center',
        gap: 1.5,
        px: 2,
        py: 1,
        borderRadius: 2,
        zIndex: 10,
      }}
    >
      <GradientButton
        onClick={onGenerate}
        loading={isGenerating}
        icon={<AutoAwesomeIcon sx={{ fontSize: 18 }} />}
      >
        Generate
      </GradientButton>

      <GradientButton
        onClick={onRun}
        loading={runStatus === 'running'}
        disabled={runStatus === 'running'}
        icon={RUN_ICON[runStatus]}
        color={runStatus === 'error' ? 'error' : 'success'}
      >
        {RUN_LABEL[runStatus]}
      </GradientButton>

      <IconButton
        size="small"
        onClick={onToggleDebug}
        aria-label="Toggle dispatch panel"
        sx={{ color: showDebug ? 'primary.main' : 'text.secondary' }}
      >
        <AccountTreeIcon sx={{ fontSize: 20 }} />
      </IconButton>
    </Paper>
  )
}

export { SubmitBar }
export type { SubmitBarProps }
