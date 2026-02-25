import Paper from '@mui/material/Paper'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import CloseIcon from '@mui/icons-material/Close'
import Divider from '@mui/material/Divider'
import { PhaseZeroSummary } from './PhaseZeroSummary'
import { ActivityFeed } from './ActivityFeed'
import { DispatchTrace } from './DispatchTrace'
import { AgentTracePanel } from './AgentTracePanel'

type DebugPanelProps = {
  readonly onClose: () => void
}

/**
 * Overlay debug sidebar anchored to the right edge of the board.
 *
 * Shows three sections:
 * 1. Phase 0 Summary — what the last submit created/updated/deleted (from HTTP response)
 * 2. Activity Feed — real-time WebSocket events from the flight recorder
 * 3. Dispatch Trace — streaming tokens and tool calls from the board dispatcher
 */
function DebugPanel({ onClose }: DebugPanelProps) {
  return (
    <Paper
      elevation={8}
      sx={{
        position: 'absolute',
        top: 0,
        right: 0,
        bottom: 0,
        width: 360,
        zIndex: 20,
        display: 'flex',
        flexDirection: 'column',
        bgcolor: 'background.paper',
        borderLeft: 1,
        borderColor: 'divider',
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          px: 1.5,
          py: 1,
          borderBottom: 1,
          borderColor: 'divider',
          flexShrink: 0,
        }}
      >
        <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
          Debug
        </Typography>
        <IconButton size="small" onClick={onClose} aria-label="Close debug panel">
          <CloseIcon sx={{ fontSize: 16 }} />
        </IconButton>
      </Box>

      {/* Phase 0 Summary */}
      <Box sx={{ flexShrink: 0, p: 1 }}>
        <PhaseZeroSummary />
      </Box>

      <Divider />

      {/* Dispatch Trace */}
      <Box sx={{ flexShrink: 0, maxHeight: '30%', overflowY: 'auto', p: 1 }}>
        <DispatchTrace />
      </Box>

      <Divider />

      {/* Agent Execution Trace */}
      <Box sx={{ flexShrink: 0, maxHeight: '30%', overflowY: 'auto', p: 1 }}>
        <AgentTracePanel />
      </Box>

      <Divider />

      {/* Activity Feed — takes remaining space */}
      <Box sx={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', p: 1 }}>
        <ActivityFeed />
      </Box>
    </Paper>
  )
}

export { DebugPanel }
export type { DebugPanelProps }
