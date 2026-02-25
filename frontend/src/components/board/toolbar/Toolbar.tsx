// ============================================================================
// Toolbar — Floating Tool Selection Bar
// ============================================================================

import Paper from '@mui/material/Paper'
import IconButton from '@mui/material/IconButton'
import Tooltip from '@mui/material/Tooltip'
import NearMeIcon from '@mui/icons-material/NearMe'
import CropSquareIcon from '@mui/icons-material/CropSquare'
import ArrowForwardIcon from '@mui/icons-material/ArrowForward'
import ZoomInIcon from '@mui/icons-material/ZoomIn'
import ZoomOutIcon from '@mui/icons-material/ZoomOut'
import CenterFocusStrongIcon from '@mui/icons-material/CenterFocusStrong'
import Divider from '@mui/material/Divider'
import type { ActiveTool } from '../elements'

type ToolbarProps = {
  readonly activeTool: ActiveTool
  readonly onToolChange: (tool: ActiveTool) => void
  readonly onZoomIn: () => void
  readonly onZoomOut: () => void
  readonly onResetZoom: () => void
}

function Toolbar({ activeTool, onToolChange, onZoomIn, onZoomOut, onResetZoom }: ToolbarProps) {
  return (
    <Paper
      elevation={2}
      sx={{
        position: 'absolute',
        top: 16,
        left: '50%',
        transform: 'translateX(-50%)',
        zIndex: 10,
        display: 'flex',
        alignItems: 'center',
        gap: 0.25,
        px: 0.5,
        py: 0.25,
        borderRadius: 2,
      }}
    >
      <ToolButton
        icon={<NearMeIcon fontSize="small" />}
        tooltip="Select (V)"
        active={activeTool === 'select'}
        onClick={() => onToolChange('select')}
      />
      <ToolButton
        icon={<CropSquareIcon fontSize="small" />}
        tooltip="Box (B)"
        active={activeTool === 'box'}
        onClick={() => onToolChange('box')}
      />
      <ToolButton
        icon={<ArrowForwardIcon fontSize="small" />}
        tooltip="Arrow (A)"
        active={activeTool === 'arrow'}
        onClick={() => onToolChange('arrow')}
      />

      <Divider orientation="vertical" flexItem sx={{ mx: 0.5 }} />

      <ToolButton icon={<ZoomInIcon fontSize="small" />} tooltip="Zoom in" onClick={onZoomIn} />
      <ToolButton icon={<ZoomOutIcon fontSize="small" />} tooltip="Zoom out" onClick={onZoomOut} />
      <ToolButton icon={<CenterFocusStrongIcon fontSize="small" />} tooltip="Reset zoom" onClick={onResetZoom} />
    </Paper>
  )
}

// ── Internal ───────────────────────────────────────────────────────────────

type ToolButtonProps = {
  readonly icon: React.ReactNode
  readonly tooltip: string
  readonly active?: boolean
  readonly onClick: () => void
}

function ToolButton({ icon, tooltip, active = false, onClick }: ToolButtonProps) {
  return (
    <Tooltip title={tooltip} arrow>
      <IconButton
        size="small"
        onClick={onClick}
        sx={{
          color: active ? 'primary.main' : 'text.secondary',
          backgroundColor: active ? 'action.selected' : 'transparent',
          '&:hover': { backgroundColor: 'action.hover' },
        }}
      >
        {icon}
      </IconButton>
    </Tooltip>
  )
}

export { Toolbar }
