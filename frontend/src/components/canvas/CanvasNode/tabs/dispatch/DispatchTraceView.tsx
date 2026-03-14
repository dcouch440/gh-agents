import { useMemo, useRef, useEffect, useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { TerminalBlock } from '@/components/primitives'
import { ToolCallCard } from './ToolCallCard'
import { buildDispatchSegments } from './traceSegments'
import type { DispatchEntry } from '@/stores/dispatchStore'

type DispatchTraceViewProps = {
  entry: DispatchEntry
}

function DispatchTraceView({ entry }: DispatchTraceViewProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const shouldAutoScrollRef = useRef(true)

  const segments = useMemo(
    () => buildDispatchSegments(entry.trace),
    // trace is append-only — length is a sufficient cache key
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [entry.trace.length]
  )

  // Track whether user has scrolled up
  useEffect(() => {
    const el = containerRef.current
    if (el === null) return

    const handleScroll = () => {
      shouldAutoScrollRef.current =
        el.scrollTop + el.clientHeight >= el.scrollHeight - 20
    }

    el.addEventListener('scroll', handleScroll)
    return () => el.removeEventListener('scroll', handleScroll)
  }, [])

  // Auto-scroll on new content
  useEffect(() => {
    const el = containerRef.current
    if (el === null || !shouldAutoScrollRef.current) return
    el.scrollTop = el.scrollHeight
  }, [segments.length, entry.trace.length])

  const isRunning = entry.status === 'running'

  return (
    <Box
      ref={containerRef}
      className="nowheel nodrag nopan"
      sx={{ flex: 1, overflowY: 'auto', px: 1, py: 0.5 }}
    >
      {segments.map((segment, i) => {
        switch (segment.type) {
          case 'text':
            return (
              <Box key={`text-${i}`}>
                <TerminalBlock content={segment.content} />
              </Box>
            )
          case 'tool':
            return (
              <ToolCallCard
                key={segment.toolId}
                toolName={segment.toolName}
                toolId={segment.toolId}
                input={segment.input}
                result={segment.result}
                status={segment.status}
              />
            )
          case 'error':
            return (
              <Box
                key={`error-${i}`}
                sx={{ bgcolor: '#f8514920', px: 1.5, py: 0.5, borderRadius: 1, my: 0.5 }}
              >
                <Typography sx={{ color: '#f85149', fontSize: 10, fontFamily: 'monospace' }}>
                  {segment.error}
                </Typography>
              </Box>
            )
          case 'phase':
            return (
              <Box
                key={`phase-${i}`}
                sx={{ display: 'flex', alignItems: 'center', gap: 1, my: 1 }}
              >
                <Box sx={{ flex: 1, height: '1px', bgcolor: 'divider' }} />
                <Typography sx={{ fontSize: 10, color: 'text.disabled', fontFamily: 'monospace', whiteSpace: 'nowrap' }}>
                  {segment.label}
                </Typography>
                <Box sx={{ flex: 1, height: '1px', bgcolor: 'divider' }} />
              </Box>
            )
          case 'system_prompt':
            return (
              <DebugMessageBlock
                key={`sp-${i}`}
                label="System Prompt"
                content={segment.content}
                agentName={segment.agentName}
              />
            )
          case 'user_message':
            return (
              <DebugMessageBlock
                key={`um-${i}`}
                label="User Message"
                content={segment.content}
                agentName={segment.agentName}
              />
            )
        }
      })}

      {isRunning && (
        <Box
          component="span"
          sx={{
            display: 'inline-block',
            '@keyframes blink': {
              '0%': { opacity: 0 },
              '100%': { opacity: 1 },
            },
            animation: 'blink 0.6s step-end infinite',
          }}
        >
          {'\u258C'}
        </Box>
      )}
    </Box>
  )
}

// ── Debug Message Block (system prompt / user message) ────────────────────

type DebugMessageBlockProps = {
  label: string
  content: string
  agentName: string | null
}

function DebugMessageBlock({ label: baseLabel, content, agentName }: DebugMessageBlockProps) {
  const [expanded, setExpanded] = useState(false)
  const label = agentName ? `${baseLabel} (${agentName})` : baseLabel

  return (
    <Box sx={{ my: 0.5, border: '1px solid', borderColor: 'divider', borderRadius: 1, overflow: 'hidden' }}>
      <Box
        onClick={() => setExpanded((v) => !v)}
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 0.5,
          px: 1,
          py: 0.5,
          cursor: 'pointer',
          bgcolor: 'action.hover',
          '&:hover': { bgcolor: 'action.selected' },
        }}
      >
        <Typography sx={{ fontSize: 10, fontFamily: 'monospace', color: 'text.secondary' }}>
          {expanded ? '▾' : '▸'} {label}
        </Typography>
      </Box>
      {expanded && (
        <Box sx={{ px: 1, py: 0.5, maxHeight: 300, overflowY: 'auto' }}>
          <Typography
            component="pre"
            sx={{
              fontSize: 10,
              fontFamily: 'monospace',
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-word',
              color: 'text.secondary',
              m: 0,
            }}
          >
            {content}
          </Typography>
        </Box>
      )}
    </Box>
  )
}

export { DispatchTraceView }
export type { DispatchTraceViewProps }
