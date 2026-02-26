import { useRef, useState, useEffect } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import { TerminalBlock } from '@/components/primitives/terminal-renderer'
import { StatusDot } from './StatusDot'
import { SkeletonLines } from './SkeletonLines'
import type { GutterCell } from './buildStepTree'
import type { StepExecutionStatus } from '@/stores/workflowExecutionStore/types'

// ── Types ───────────────────────────────────────────────────────────────────

type StepTreeRowProps = {
  readonly name: string
  readonly stepId: string
  readonly executionMode: string
  readonly gutter: readonly GutterCell[]
  readonly status: StepExecutionStatus | undefined
  readonly output: string | null
  readonly error: string | null
  readonly isExpanded: boolean
  readonly isOutputExpanded: boolean
  readonly onToggle: () => void
  readonly onToggleOutputExpand: () => void
}

// ── Gutter Line Rendering ──────────────────────────────────────────────────

const CELL_WIDTH = 28

/** x-offset of vertical lines within each cell column */
const LINE_X = 4

/** Line thickness in pixels — string to prevent MUI spacing interpretation */
const STROKE = '1px'

type LineDef = {
  readonly left: number
  readonly top: number | string
  readonly width: number | string
  readonly height: number | string
}

/**
 * Compute CSS line segments for the header gutter.
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
    const halfDown = cell === 'root_fork' || cell === 'fork_start'

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

// ── Output Preview ──────────────────────────────────────────────────────────

const PREVIEW_MAX_HEIGHT = 200

type OutputPreviewProps = {
  readonly output: string
  readonly isOutputExpanded: boolean
  readonly onToggleOutputExpand: () => void
  readonly bgColor: string
}

function OutputPreview({ output, isOutputExpanded, onToggleOutputExpand, bgColor }: OutputPreviewProps) {
  const contentRef = useRef<HTMLDivElement>(null)
  const [overflows, setOverflows] = useState(false)

  useEffect(() => {
    const el = contentRef.current
    if (!el || isOutputExpanded) return
    setOverflows(el.scrollHeight > el.clientHeight)
  }, [output, isOutputExpanded])

  return (
    <Box sx={{ position: 'relative', py: 1 }}>
      <Box
        ref={contentRef}
        sx={{
          ...(isOutputExpanded ? {} : { maxHeight: PREVIEW_MAX_HEIGHT, overflow: 'hidden' }),
        }}
      >
        <TerminalBlock content={output} />
      </Box>

      {/* Fade gradient — only when content actually overflows */}
      {!isOutputExpanded && overflows && (
        <Box
          sx={{
            position: 'absolute',
            bottom: 8,
            left: 0,
            right: 0,
            height: 48,
            background: `linear-gradient(transparent, ${bgColor})`,
            pointerEvents: 'none',
          }}
        />
      )}

      {/* Expand/collapse link — only when content overflows or already expanded */}
      {(overflows || isOutputExpanded) && (
        <Typography
          component="span"
          onClick={(e) => {
            e.stopPropagation()
            onToggleOutputExpand()
          }}
          sx={{
            display: 'block',
            fontSize: 11,
            color: 'text.disabled',
            cursor: 'pointer',
            mt: 0.5,
            '&:hover': { color: 'text.secondary' },
          }}
        >
          {isOutputExpanded ? '\u25B2 collapse' : '\u25BC expand'}
        </Typography>
      )}
    </Box>
  )
}

// ── Component ───────────────────────────────────────────────────────────────

function StepTreeRow({
  name,
  gutter,
  status,
  output,
  error,
  isExpanded,
  isOutputExpanded,
  onToggle,
  onToggleOutputExpand,
}: StepTreeRowProps) {
  const theme = useTheme()
  const lines = computeLines(gutter)
  const lineColor = theme.palette.text.disabled
  const gutterWidth = gutter.length * CELL_WIDTH

  const resolved = status ?? 'idle'
  const hasBody = isExpanded && (resolved === 'running' || output !== null || error !== null)

  return (
    <Box>
      {/* Header row */}
      <Box
        role="treeitem"
        onClick={onToggle}
        sx={{
          display: 'flex',
          alignItems: 'center',
          pl: '8px',
          pr: 1,
          py: '5px',
          cursor: 'pointer',
          '&:hover': { backgroundColor: theme.palette.custom.hoverOverlay },
        }}
      >
        {/* Gutter */}
        <Box
          sx={{
            width: gutterWidth,
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

        {/* Expand chevron */}
        <Typography
          sx={{
            fontSize: 10,
            width: 12,
            flexShrink: 0,
            color: 'text.disabled',
            lineHeight: 1,
            userSelect: 'none',
          }}
        >
          {isExpanded ? '\u25BC' : '\u25B6'}
        </Typography>

        {/* Step name */}
        <Typography
          variant="body2"
          sx={{
            fontSize: 12,
            fontWeight: 400,
            color: 'text.secondary',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            minWidth: 0,
            flex: 1,
            ml: 0.5,
          }}
        >
          {name || 'Untitled'}
        </Typography>

        {/* Status dot */}
        <Box sx={{ ml: 1, flexShrink: 0 }}>
          <StatusDot status={status} />
        </Box>
      </Box>

      {/* Output body */}
      {hasBody && (
        <Box sx={{ display: 'flex', pl: '8px', mt: '-5px' }}>
          {/* Continuation gutter — mt: -5px closes the gap left by the header's py padding */}
          <Box
            sx={{
              width: gutterWidth,
              flexShrink: 0,
              position: 'relative',
              overflow: 'visible',
            }}
          >
            {computeContinuationLines(gutter).map((line, i) => (
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

          {/* Content */}
          <Box sx={{ flex: 1, minWidth: 0, pr: 1, pb: 1, ml: 1.5 }}>
            {resolved === 'running' ? (
              <SkeletonLines />
            ) : error ? (
              <Typography
                variant="body2"
                sx={{
                  color: '#f85149',
                  fontFamily: '"JetBrains Mono", monospace',
                  fontSize: '0.75rem',
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word',
                  py: 1,
                }}
              >
                {error}
              </Typography>
            ) : output ? (
              <OutputPreview
                output={output}
                isOutputExpanded={isOutputExpanded}
                onToggleOutputExpand={onToggleOutputExpand}
                bgColor={theme.palette.custom.bgPanel}
              />
            ) : null}
          </Box>
        </Box>
      )}
    </Box>
  )
}

export { StepTreeRow, CELL_WIDTH, LINE_X }
export type { StepTreeRowProps }
