import { useState } from 'react'
import Paper from '@mui/material/Paper'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import CloseIcon from '@mui/icons-material/Close'
import { DispatchTab } from './DispatchTab'
import { RunTab } from './RunTab'

type DispatchPanelProps = {
  readonly onClose: () => void
}

type PanelTab = 'dispatch' | 'run'

/**
 * Side panel overlay with tabbed navigation between Dispatch and Run activity.
 */
function DispatchPanel({ onClose }: DispatchPanelProps) {
  const [activeTab, setActiveTab] = useState<PanelTab>('dispatch')

  return (
    <Paper
      elevation={8}
      sx={{
        position: 'absolute',
        top: 0,
        right: 0,
        bottom: 0,
        width: 400,
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
          flexShrink: 0,
        }}
      >
        <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
          Activity
        </Typography>
        <IconButton size="small" onClick={onClose} aria-label="Close panel">
          <CloseIcon sx={{ fontSize: 16 }} />
        </IconButton>
      </Box>

      {/* Tab bar */}
      <Box sx={{ display: 'flex', borderBottom: 1, borderColor: 'divider', flexShrink: 0 }}>
        <TabButton label="Dispatch" active={activeTab === 'dispatch'} onClick={() => setActiveTab('dispatch')} />
        <TabButton label="Run" active={activeTab === 'run'} onClick={() => setActiveTab('run')} />
      </Box>

      {/* Content */}
      <Box sx={{ flex: 1, minHeight: 0, overflowY: 'auto' }}>
        {activeTab === 'dispatch' ? <DispatchTab /> : <RunTab />}
      </Box>
    </Paper>
  )
}

// ── Tab button ───────────────────────────────────────────────────────────────

type TabButtonProps = {
  readonly label: string
  readonly active: boolean
  readonly onClick: () => void
}

function TabButton({ label, active, onClick }: TabButtonProps) {
  return (
    <Box
      onClick={onClick}
      sx={{
        flex: 1,
        textAlign: 'center',
        py: 0.75,
        cursor: 'pointer',
        fontFamily: 'monospace',
        fontSize: 12,
        fontWeight: active ? 600 : 400,
        color: active ? 'text.primary' : 'text.secondary',
        borderBottom: 2,
        borderColor: active ? 'primary.main' : 'transparent',
        '&:hover': { bgcolor: 'action.hover' },
        userSelect: 'none',
      }}
    >
      {label}
    </Box>
  )
}

export { DispatchPanel }
export type { DispatchPanelProps }
