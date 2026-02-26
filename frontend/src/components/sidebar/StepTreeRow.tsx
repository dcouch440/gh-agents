import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import { ANIMATION } from '@/constants'
import type { GutterCell } from './buildStepTree'

// ── Types ───────────────────────────────────────────────────────────────────

type StepTreeRowProps = {
  readonly name: string
  readonly executionMode: string
  readonly gutter: readonly GutterCell[]
  readonly isSelected: boolean
  readonly onClick: () => void
}

// ── Gutter Line Rendering ──────────────────────────────────────────────────

const CELL_WIDTH = 28

/** x-offset of vertical lines within each cell column */
const LINE_X = 4

/** Line thickness in pixels — string to prevent MUI spacing interpretation */
const STROKE = '1px'

/**
 * Each line segment is an absolutely-positioned 1px div.
 * Percentage strings ('50%', '100%') are used for y-axis values
 * so lines scale with row height and span through padding.
 */
type LineDef = {
  readonly left: number
  readonly top: number | string
  readonly width: number | string
  readonly height: number | string
}

/**
 * Compute CSS line segments for the entire gutter.
 *
 * Cells that need vertical continuation (pipe, branch, fork_start, par_mid)
 * draw full-height vertical lines. Corner/par_end draw half-height (top→center).
 * Horizontal lines extend from the junction point to the gutter's right edge.
 * Fork junctions draw a vertical line at the next column's position going
 * down from center — this overflows the gutter container slightly.
 */
const computeLines = (gutter: readonly GutterCell[]): LineDef[] => {
  const lines: LineDef[] = []
  const rightEdge = gutter.length * CELL_WIDTH

  for (let i = 0; i < gutter.length; i++) {
    const cell = gutter[i]!
    const cx = i * CELL_WIDTH + LINE_X

    if (cell === 'blank') continue

    // ── Vertical lines ───────────────────────────────────────────────────
    const fullVert = cell === 'pipe' || cell === 'branch' || cell === 'par_mid'
    const halfUp = cell === 'corner' || cell === 'par_end'
    const halfDown = cell === 'root_fork' || cell === 'fork_start'

    if (fullVert) {
      lines.push({ left: cx, top: 0, width: STROKE, height: '100%' })
    } else if (halfUp) {
      lines.push({ left: cx, top: 0, width: STROKE, height: '50%' })
    } else if (halfDown) {
      lines.push({ left: cx, top: '50%', width: STROKE, height: '50%' })
    }

    // ── Horizontal lines (all visible cells except pipe) ─────────────────
    // Stop 4px short of the gutter edge so the line doesn't collide with the dot
    if (cell !== 'pipe') {
      lines.push({ left: cx, top: '50%', width: rightEdge - cx, height: STROKE })
    }

  }

  return lines
}

// ── Component ───────────────────────────────────────────────────────────────

function StepTreeRow({ name, gutter, isSelected, onClick }: StepTreeRowProps) {
  const theme = useTheme()
  const lines = computeLines(gutter)
  const lineColor = theme.palette.text.disabled

  return (
    <Box
      role="treeitem"
      aria-selected={isSelected}
      onClick={onClick}
      sx={{
        display: 'flex',
        alignItems: 'center',
        pl: '8px',
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
      {/* Gutter — CSS-drawn lines that span the full row height including padding.
          Negative vertical margin extends the gutter into the row's py padding
          so vertical lines connect continuously between adjacent rows. */}
      <Box
        sx={{
          width: gutter.length * CELL_WIDTH,
          flexShrink: 0,
          alignSelf: 'stretch',
          position: 'relative',
          overflow: 'visible',
          my: '-5px',
        }}
      >
        {lines.map((line, i) => (
          <Box
            key={i}
            sx={{
              position: 'absolute',
              left: line.left,
              top: line.top,
              width: line.width,
              height: line.height,
              backgroundColor: lineColor,
            }}
          />
        ))}
      </Box>

      {/* Mode dot */}

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
          ml: 1,
        }}
      >
        {name || 'Untitled'}
      </Typography>
    </Box>
  )
}

export { StepTreeRow }
export type { StepTreeRowProps }
