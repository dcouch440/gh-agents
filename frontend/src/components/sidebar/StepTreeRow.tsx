import { useRef, useState, useEffect } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import PushPinIcon from '@mui/icons-material/PushPin'
import PushPinOutlinedIcon from '@mui/icons-material/PushPinOutlined'
import { TerminalBlock } from '@/components/primitives/terminal-renderer'
import { designStatusColor } from '@/utils/statusColor'
import { StatusDot } from './StatusDot'
import { SkeletonLines } from './SkeletonLines'
import { CELL_WIDTH, LINE_X, STROKE, HEADER_CENTER, computeLines } from './gutterLines'
import type { GutterCell } from './buildStepTree'
import type { StepExecutionStatus } from '@/stores/workflowExecutionStore/types'
import type { SourceStreamStatus } from '@/stores/stepStreamStore'

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
  readonly designStatus?: SourceStreamStatus | null
  /** Design-phase line: the live phase marker, or the reason a design failed. */
  readonly designProgress?: string | null
  readonly pinned: boolean
  readonly onTogglePin: () => void
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
  executionMode,
  gutter,
  status,
  output,
  error,
  isExpanded,
  isOutputExpanded,
  onToggle,
  onToggleOutputExpand,
  designStatus,
  designProgress,
  pinned,
  onTogglePin,
}: StepTreeRowProps) {
  const theme = useTheme()
  const statusPalette = theme.palette.statusPalette
  const resolved = status ?? 'idle'
  // Only the two design states with something to say get a line of their own.
  const designMessageColor =
    designStatus === 'running' || designStatus === 'failed'
      ? designStatusColor(designStatus, statusPalette)
      : null
  const isWorkforce = executionMode === 'workforce'
  const lines = computeLines(gutter)
  const lineColor = theme.palette.text.disabled
  const gutterWidth = gutter.length * CELL_WIDTH
  const hasBody = isExpanded && !isWorkforce && (resolved === 'running' || output !== null || error !== null)

  return (
    <Box
      sx={{
        display: 'flex',
        pl: '8px',
        '&:hover': { backgroundColor: theme.palette.custom.hoverOverlay },
        '&:hover .pin-toggle': { opacity: 1 },
      }}
    >
      {/* Single gutter — spans full row height (header + body) */}
      <Box
        sx={{
          width: gutterWidth,
          flexShrink: 0,
          alignSelf: 'stretch',
          position: 'relative',
          overflow: 'visible',
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
        {/* Corner bend for workforce steps — connects branch to agents below */}
        {isWorkforce && (
          <>
            {/* Horizontal bridge: end of branch → corner vertical */}
            <Box
              sx={{
                position: 'absolute',
                left: gutterWidth,
                top: HEADER_CENTER,
                width: LINE_X + 1,
                height: STROKE,
                backgroundColor: lineColor,
              }}
            />
            {/* Vertical: midpoint → bottom, connects to agent gutter */}
            <Box
              sx={{
                position: 'absolute',
                left: gutterWidth + LINE_X,
                top: HEADER_CENTER,
                bottom: 0,
                width: STROKE,
                backgroundColor: lineColor,
              }}
            />
          </>
        )}
      </Box>

      {/* Content column — header + optional body stacked vertically */}
      <Box sx={{ flex: 1, minWidth: 0 }}>
        {/* Header row */}
        <Box
          role="treeitem"
          onClick={onToggle}
          sx={{
            display: 'flex',
            alignItems: 'center',
            pr: 1,
            py: '5px',
            cursor: 'pointer',
          }}
        >
          {/* Expand chevron (spacer for workforce — agents always visible) */}
          {isWorkforce ? (
            <Box sx={{ width: 16, flexShrink: 0 }} />
          ) : (
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
          )}

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

          {/* Latest dispatch phase marker (e.g. "designing agents"), or, once a
              design has failed, why — otherwise the row went red in silence. */}
          {designMessageColor !== null && designProgress !== null && designProgress !== '' && (
            <Typography
              title={designProgress}
              sx={{
                fontSize: 10,
                color: designMessageColor,
                fontFamily: 'monospace',
                flexShrink: 0,
                ml: 0.5,
                maxWidth: 120,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              {designProgress}
            </Typography>
          )}

          {/* Pin toggle — visible on hover, always visible when pinned */}
          <Box
            component="span"
            className="pin-toggle"
            onClick={(e) => { e.stopPropagation(); onTogglePin() }}
            sx={{
              display: 'inline-flex',
              flexShrink: 0,
              ml: 0.5,
              opacity: pinned ? 1 : 0,
              color: pinned ? '#58a6ff' : 'text.disabled',
              cursor: 'pointer',
              transition: 'opacity 0.15s',
              '&:hover': { color: '#58a6ff' },
            }}
          >
            {pinned
              ? <PushPinIcon sx={{ fontSize: 12 }} />
              : <PushPinOutlinedIcon sx={{ fontSize: 12 }} />}
          </Box>

          {/* Status dot */}
          <Box sx={{ ml: 1, flexShrink: 0 }}>
            <StatusDot status={status} designStatus={designStatus} pinned={pinned} />
          </Box>
        </Box>

        {/* Output body */}
        {hasBody && (
          <Box sx={{ pr: 1, pb: 1, ml: 1.5 }}>
            {resolved === 'running' ? (
              <SkeletonLines />
            ) : error ? (
              <Typography
                variant="body2"
                sx={{
                  color: statusPalette.failed,
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
        )}
      </Box>
    </Box>
  )
}

export { StepTreeRow }
export type { StepTreeRowProps }
