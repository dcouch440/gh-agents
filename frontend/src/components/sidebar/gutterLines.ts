import type { GutterCell } from './buildStepTree'

// ── Constants ───────────────────────────────────────────────────────────────

const CELL_WIDTH = 28

/** x-offset of vertical lines within each cell column */
const LINE_X = 4

/** Line thickness in pixels — string to prevent MUI spacing interpretation */
const STROKE = '1px'

/**
 * Fixed pixel offset for the header row's vertical center.
 * Used instead of '50%' so horizontal stubs align to the header
 * even when the gutter spans a taller row (header + expanded body).
 *
 * Derived from: py(5px) + half of content height (~10px) = 15px
 */
const HEADER_CENTER = 15

// ── Types ───────────────────────────────────────────────────────────────────

type LineDef = {
  readonly left: number
  readonly top: number | string
  readonly width: number | string
  readonly height: number | string
}

// ── Gutter Computation ──────────────────────────────────────────────────────

/**
 * Compute CSS line segments for a row's gutter.
 *
 * Horizontal stubs are anchored at HEADER_CENTER (fixed px) so they
 * always point to the header label, even when the gutter spans a
 * taller row with expanded output below.
 */
const computeLines = (gutter: readonly GutterCell[]): LineDef[] => {
  const lines: LineDef[] = []
  const rightEdge = gutter.length * CELL_WIDTH

  for (let i = 0; i < gutter.length; i++) {
    const cell = gutter[i]!
    const cx = i * CELL_WIDTH + LINE_X

    if (cell === 'blank') continue

    const fullVert = cell === 'pipe' || cell === 'branch' || cell === 'par_mid' || cell === 'fork_start'
    const halfUp = cell === 'corner' || cell === 'par_end'

    if (fullVert) {
      lines.push({ left: cx, top: 0, width: STROKE, height: '100%' })
    } else if (halfUp) {
      lines.push({ left: cx, top: 0, width: STROKE, height: HEADER_CENTER })
    }

    if (cell !== 'pipe') {
      lines.push({ left: cx, top: HEADER_CENTER, width: rightEdge - cx, height: STROKE })
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

export { CELL_WIDTH, LINE_X, STROKE, HEADER_CENTER, computeLines, toContinuationGutter }
export type { LineDef }
