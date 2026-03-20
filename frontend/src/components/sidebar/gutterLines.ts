import type { GutterCell } from './buildStepTree'

// ── Constants ───────────────────────────────────────────────────────────────

const CELL_WIDTH = 28

/** x-offset of vertical lines within each cell column */
const LINE_X = 4

/** Line thickness in pixels — string to prevent MUI spacing interpretation */
const STROKE = '1px'

// ── Types ───────────────────────────────────────────────────────────────────

type LineDef = {
  readonly left: number
  readonly top: number | string
  readonly width: number | string
  readonly height: number | string
}

// ── Gutter Computation ──────────────────────────────────────────────────────

/**
 * Compute CSS line segments for a header gutter row.
 */
const computeLines = (gutter: readonly GutterCell[]): LineDef[] => {
  const lines: LineDef[] = []
  const rightEdge = gutter.length * CELL_WIDTH

  for (let i = 0; i < gutter.length; i++) {
    const cell = gutter[i]!
    const cx = i * CELL_WIDTH + LINE_X

    if (cell === 'blank') continue

    const fullVert = cell === 'pipe' || cell === 'branch' || cell === 'par_mid'
    const halfUp = cell === 'corner' || cell === 'par_end'
    const halfDown = cell === 'fork_start'

    if (fullVert) {
      lines.push({ left: cx, top: 0, width: STROKE, height: '100%' })
    } else if (halfUp) {
      lines.push({ left: cx, top: 0, width: STROKE, height: '50%' })
    } else if (halfDown) {
      lines.push({ left: cx, top: '50%', width: STROKE, height: '50%' })
    }

    if (cell !== 'pipe') {
      lines.push({ left: cx, top: '50%', width: rightEdge - cx, height: STROKE })
    }
  }

  return lines
}

/**
 * Derive continuation gutter from header gutter.
 * Cells that need vertical continuation become 'pipe', others become 'blank'.
 */
const toContinuationGutter = (gutter: readonly GutterCell[]): GutterCell[] =>
  gutter.map((cell) => {
    if (cell === 'pipe' || cell === 'branch' || cell === 'fork_start' || cell === 'par_mid') {
      return 'pipe'
    }
    return 'blank'
  })

/**
 * Compute CSS line segments for the continuation gutter (output body).
 * Only full-height vertical lines — no horizontals, no half-heights.
 */
const computeContinuationLines = (gutter: readonly GutterCell[]): LineDef[] => {
  const continuation = toContinuationGutter(gutter)
  const lines: LineDef[] = []

  for (let i = 0; i < continuation.length; i++) {
    if (continuation[i] === 'pipe') {
      lines.push({ left: i * CELL_WIDTH + LINE_X, top: 0, width: STROKE, height: '100%' })
    }
  }

  return lines
}

export { CELL_WIDTH, LINE_X, STROKE, computeLines, toContinuationGutter, computeContinuationLines }
export type { LineDef }
