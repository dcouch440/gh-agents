import { useRef, useState, useEffect } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import { TerminalBlock } from '@/components/primitives/terminal-renderer'
import { CELL_WIDTH, computeLines, computeContinuationLines } from './gutterLines'
import { StatusDot } from './StatusDot'
import { SkeletonLines } from './SkeletonLines'
import type { GutterCell } from './buildStepTree'
import type { StepExecutionStatus } from '@/stores/workflowExecutionStore/types'
import type { SourceStreamStatus } from '@/stores/stepStreamStore'

// ── Types ───────────────────────────────────────────────────────────────────

type AgentTreeRowProps = {
  readonly agentName: string
  readonly agentId: string
  readonly stepId: string
  readonly gutter: readonly GutterCell[]
  readonly output: string | null
  readonly isExpanded: boolean
  readonly onToggle: () => void
  readonly designStatus: SourceStreamStatus | null
  readonly executionStatus: SourceStreamStatus | null
}

const toExecStatus = (s: SourceStreamStatus | null): StepExecutionStatus | undefined => {
  if (s === null) return undefined
  if (s === 'completed') return 'success'
  if (s === 'failed') return 'error'
  return s
}

// ── Output Preview ──────────────────────────────────────────────────────────

const PREVIEW_MAX_HEIGHT = 200

type AgentOutputPreviewProps = {
  readonly output: string
  readonly bgColor: string
}

function AgentOutputPreview({ output, bgColor }: AgentOutputPreviewProps) {
  const contentRef = useRef<HTMLDivElement>(null)
  const [overflows, setOverflows] = useState(false)
  const [isExpanded, setIsExpanded] = useState(false)

  useEffect(() => {
    const el = contentRef.current
    if (!el || isExpanded) return
    setOverflows(el.scrollHeight > el.clientHeight)
  }, [output, isExpanded])

  return (
    <Box sx={{ position: 'relative', py: 1 }}>
      <Box
        ref={contentRef}
        sx={{
          ...(isExpanded ? {} : { maxHeight: PREVIEW_MAX_HEIGHT, overflow: 'hidden' }),
        }}
      >
        <TerminalBlock content={output} />
      </Box>

      {!isExpanded && overflows && (
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

      {(overflows || isExpanded) && (
        <Typography
          component="span"
          onClick={(e) => {
            e.stopPropagation()
            setIsExpanded((prev) => !prev)
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
          {isExpanded ? '\u25B2 collapse' : '\u25BC expand'}
        </Typography>
      )}
    </Box>
  )
}

// ── Component ───────────────────────────────────────────────────────────────

function AgentTreeRow({
  agentName,
  gutter,
  output,
  isExpanded,
  onToggle,
  designStatus,
  executionStatus,
}: AgentTreeRowProps) {
  const theme = useTheme()
  const lines = computeLines(gutter)
  const lineColor = theme.palette.text.disabled
  const gutterWidth = gutter.length * CELL_WIDTH

  const isRunning = executionStatus === 'running'
  const hasBody = isExpanded && (isRunning || output !== null)

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

        {/* Agent name */}
        <Typography
          variant="body2"
          sx={{
            fontSize: 12,
            fontWeight: 400,
            color: 'text.disabled',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            minWidth: 0,
            flex: 1,
            ml: 0.5,
            fontStyle: 'italic',
          }}
        >
          {agentName}
        </Typography>

        {/* Status dot */}
        <Box sx={{ ml: 1, flexShrink: 0 }}>
          <StatusDot status={toExecStatus(executionStatus)} designStatus={designStatus} />
        </Box>
      </Box>

      {/* Output body */}
      {hasBody && (
        <Box sx={{ display: 'flex', pl: '8px', mt: '-5px' }}>
          {/* Continuation gutter */}
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
            {isRunning ? (
              <SkeletonLines />
            ) : output !== null ? (
              <AgentOutputPreview
                output={output}
                bgColor={theme.palette.custom.bgPanel}
              />
            ) : null}
          </Box>
        </Box>
      )}
    </Box>
  )
}

export { AgentTreeRow }
export type { AgentTreeRowProps }
