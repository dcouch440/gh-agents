import { useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Menu from '@mui/material/Menu'
import MenuItem from '@mui/material/MenuItem'
import ListItemText from '@mui/material/ListItemText'
import ListItemIcon from '@mui/material/ListItemIcon'
import CircularProgress from '@mui/material/CircularProgress'
import KeyboardArrowDownOutlined from '@mui/icons-material/KeyboardArrowDownOutlined'
import FiberManualRecordOutlined from '@mui/icons-material/FiberManualRecordOutlined'
import CheckCircleOutlined from '@mui/icons-material/CheckCircleOutlined'
import ErrorOutlineOutlined from '@mui/icons-material/ErrorOutlineOutlined'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import type { WorkflowExecutionSummary } from '@/types'
import { formatRelativeTime } from '@/utils/formatRelativeTime'

type ExecutionRunSelectorProps = {
  currentRunId: string | null
  runs: WorkflowExecutionSummary[]
  selectedHistoricalRunId: string | null
  isRunning: boolean
  loading: boolean
  onSelectRun: (runId: string) => void
  onReturnToLive: () => void
}

const statusIcon = (status: string) => {
  if (status === 'completed') return <CheckCircleOutlined sx={{ fontSize: 14, color: 'success.main' }} />
  if (status === 'failed') return <ErrorOutlineOutlined sx={{ fontSize: 14, color: 'error.main' }} />
  return <FiberManualRecordOutlined sx={{ fontSize: 14, color: 'text.disabled' }} />
}

function ExecutionRunSelector({
  currentRunId,
  runs,
  selectedHistoricalRunId,
  isRunning,
  loading,
  onSelectRun,
  onReturnToLive,
}: ExecutionRunSelectorProps) {
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
  const open = Boolean(anchorEl)

  const activeId = selectedHistoricalRunId ?? currentRunId
  if (!activeId && runs.length === 0) return null

  const displayLabel = activeId ? `Run ${activeId.slice(0, 8)}` : 'Select run'

  return (
    <>
      <Box
        onClick={(e) => setAnchorEl(e.currentTarget)}
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 0.75,
          px: 2,
          py: 0.75,
          borderBottom: 1,
          borderColor: 'divider',
          cursor: 'pointer',
          '&:hover': { bgcolor: 'action.hover' },
        }}
      >
        {isRunning && !selectedHistoricalRunId && (
          <Box
            sx={{
              width: 6,
              height: 6,
              borderRadius: '50%',
              bgcolor: 'success.main',
              animation: 'pulse 1.5s infinite',
              '@keyframes pulse': {
                '0%, 100%': { opacity: 1 },
                '50%': { opacity: 0.4 },
              },
            }}
          />
        )}
        <Typography variant="caption" sx={{ color: 'text.secondary', fontFamily: 'monospace', flex: 1 }}>
          {displayLabel}
        </Typography>
        {loading ? <CircularProgress size={12} /> : <KeyboardArrowDownOutlined sx={{ fontSize: 16, color: 'text.disabled' }} />}
      </Box>
      <Menu
        anchorEl={anchorEl}
        open={open}
        onClose={() => setAnchorEl(null)}
        slotProps={{ paper: { sx: { maxHeight: 280, minWidth: 220 } } }}
      >
        {currentRunId && selectedHistoricalRunId && (
          <MenuItem
            onClick={() => {
              onReturnToLive()
              setAnchorEl(null)
            }}
          >
            <ListItemIcon>
              <PlayArrowOutlined sx={{ fontSize: 16 }} />
            </ListItemIcon>
            <ListItemText
              primary={
                <Typography variant="caption" sx={{ fontWeight: 600 }}>
                  Back to live
                </Typography>
              }
            />
          </MenuItem>
        )}
        {runs.map((run) => {
          const isActive = run.id === activeId
          return (
            <MenuItem
              key={run.id}
              selected={isActive}
              onClick={() => {
                onSelectRun(run.id)
                setAnchorEl(null)
              }}
            >
              <ListItemIcon>{statusIcon(run.status)}</ListItemIcon>
              <ListItemText
                primary={
                  <Typography variant="caption" sx={{ fontFamily: 'monospace' }}>
                    {run.id.slice(0, 8)}
                  </Typography>
                }
                secondary={formatRelativeTime(run.started_at)}
              />
            </MenuItem>
          )
        })}
        {runs.length === 0 && !loading && (
          <MenuItem disabled>
            <ListItemText primary="No runs yet" />
          </MenuItem>
        )}
      </Menu>
    </>
  )
}

export { ExecutionRunSelector }
export type { ExecutionRunSelectorProps }
