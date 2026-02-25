import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import { ANIMATION } from '@/constants'

// ── Types ───────────────────────────────────────────────────────────────────

type StepTreeRowProps = {
  readonly name: string
  readonly executionMode: string
  readonly depth: number
  readonly isLast: boolean
  readonly isSelected: boolean
  readonly onClick: () => void
}

// ── Constants ───────────────────────────────────────────────────────────────

const DEPTH_INDENT = 20
const CONNECTOR_WIDTH = 28

const getModeColor = (mode: string, palette: Record<string, string>): string =>
  palette[mode] ?? palette.step ?? '#888'

// ── Component ───────────────────────────────────────────────────────────────

function StepTreeRow({ name, executionMode, depth, isLast, isSelected, onClick }: StepTreeRowProps) {
  const theme = useTheme()
  const connector = isLast ? '\u2514\u2500\u2500' : '\u251C\u2500\u2500'
  const modeColor = getModeColor(executionMode, theme.palette.nodePalette)

  return (
    <Box
      role="treeitem"
      aria-selected={isSelected}
      onClick={onClick}
      sx={{
        display: 'flex',
        alignItems: 'center',
        pl: `${depth * DEPTH_INDENT + 8}px`,
        pr: 1,
        py: '5px',
        cursor: 'pointer',
        borderLeft: isSelected ? `2px solid ${theme.palette.primary.main}` : '2px solid transparent',
        backgroundColor: isSelected ? theme.palette.custom.activeTint : 'transparent',
        transition: `all ${ANIMATION.FAST}ms ease`,
        '&:hover': isSelected
          ? {}
          : { backgroundColor: theme.palette.custom.hoverOverlay },
      }}
    >
      {/* Connector character */}
      <Typography
        component="span"
        sx={{
          fontFamily: '"JetBrains Mono", monospace',
          fontSize: 11,
          lineHeight: 1,
          color: 'text.disabled',
          width: CONNECTOR_WIDTH,
          flexShrink: 0,
          userSelect: 'none',
        }}
      >
        {connector}
      </Typography>

      {/* Mode dot */}
      <Box
        sx={{
          width: 6,
          height: 6,
          borderRadius: '50%',
          backgroundColor: modeColor,
          flexShrink: 0,
          mr: 0.75,
        }}
      />

      {/* Step name */}
      <Typography
        variant="body2"
        sx={{
          fontSize: 12,
          fontWeight: isSelected ? 600 : 400,
          color: isSelected ? 'text.primary' : 'text.secondary',
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          minWidth: 0,
          flex: 1,
        }}
      >
        {name || 'Untitled'}
      </Typography>
    </Box>
  )
}

export { StepTreeRow }
export type { StepTreeRowProps }
