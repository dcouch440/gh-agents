import { useState, useMemo, useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import ExpandMoreIcon from '@mui/icons-material/ExpandMore'
import ExpandLessIcon from '@mui/icons-material/ExpandLess'
import { ToolCallCard } from '@/components/canvas/CanvasNode/tabs/dispatch/ToolCallCard'
import type { AgentTrace, AgentTraceEvent } from '@/stores/agentTraceStore'

type AgentTraceCardProps = {
  readonly trace: AgentTrace
}

const mono = {
  fontFamily: 'monospace',
  fontSize: 11,
  lineHeight: 1.5,
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',
} as const

/**
 * Collapsible card showing a single agent's execution trace.
 * Renders system prompt, input, tool calls (via ToolCallCard), and output.
 */
function AgentTraceCard({ trace }: AgentTraceCardProps) {
  const [expanded, setExpanded] = useState(false)
  const toggle = useCallback(() => setExpanded((v) => !v), [])

  const agentLabel = trace.agentName ?? trace.agentExecutionId.slice(0, 8)
  const toolCallCount = trace.events.filter((e) => e.type === 'tool_call').length

  return (
    <Box sx={{ borderBottom: 1, borderColor: 'divider', '&:last-child': { borderBottom: 0 } }}>
      {/* Header */}
      <Box
        onClick={toggle}
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 1,
          px: 1.5,
          py: 0.75,
          cursor: 'pointer',
          '&:hover': { bgcolor: 'action.hover' },
        }}
      >
        <IconButton size="small" sx={{ p: 0, flexShrink: 0 }}>
          {expanded
            ? <ExpandLessIcon sx={{ fontSize: 16, color: 'text.secondary' }} />
            : <ExpandMoreIcon sx={{ fontSize: 16, color: 'text.secondary' }} />}
        </IconButton>

        <Typography
          sx={{
            fontFamily: 'monospace',
            fontSize: 12,
            fontWeight: 600,
            color: 'text.primary',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            flex: 1,
            minWidth: 0,
          }}
        >
          {agentLabel}
        </Typography>

        {toolCallCount > 0 && (
          <Typography sx={{ fontSize: 10, color: 'text.disabled', fontFamily: 'monospace', flexShrink: 0 }}>
            {toolCallCount} tool(s)
          </Typography>
        )}
      </Box>

      {/* Expanded detail */}
      {expanded && <AgentTraceDetail trace={trace} />}
    </Box>
  )
}

// ── Detail view ──────────────────────────────────────────────────────────────

function AgentTraceDetail({ trace }: { readonly trace: AgentTrace }) {
  // Build tool_result lookup so we can pair results with their calls
  const resultMap = useMemo(() => {
    const map = new Map<string, string>()
    for (const e of trace.events) {
      if (e.type === 'tool_result') {
        map.set(e.toolId, e.result)
      }
    }
    return map
  }, [trace.events])

  return (
    <Box sx={{ px: 1.5, py: 0.5, borderTop: 1, borderColor: 'divider' }}>
      {trace.events.map((event, i) => (
        <TraceEventRow key={i} event={event} resultMap={resultMap} />
      ))}
    </Box>
  )
}

// ── Individual event renderers ───────────────────────────────────────────────

type TraceEventRowProps = {
  readonly event: AgentTraceEvent
  readonly resultMap: ReadonlyMap<string, string>
}

function TraceEventRow({ event, resultMap }: TraceEventRowProps) {
  switch (event.type) {
    case 'system_prompt':
      return <CollapsibleSection label="System Prompt" content={event.content} />
    case 'user_message':
      return <CollapsibleSection label="Input" content={event.content} defaultOpen />
    case 'assistant_message':
      return <CollapsibleSection label="Output" content={event.content} defaultOpen />
    case 'tool_call':
      return (
        <ToolCallCard
          toolName={event.toolName}
          toolId={event.toolId}
          input={event.input}
          result={resultMap.get(event.toolId) ?? null}
          status={resultMap.has(event.toolId) ? 'complete' : 'running'}
        />
      )
    case 'tool_result':
      // Consumed by tool_call pairing above
      return null
  }
}

// ── Collapsible section ──────────────────────────────────────────────────────

type CollapsibleSectionProps = {
  readonly label: string
  readonly content: string
  readonly defaultOpen?: boolean
}

function CollapsibleSection({ label, content, defaultOpen = false }: CollapsibleSectionProps) {
  const [open, setOpen] = useState(defaultOpen)

  return (
    <Box sx={{ mb: 0.5 }}>
      <Box
        onClick={() => setOpen((v) => !v)}
        sx={{ display: 'flex', alignItems: 'center', cursor: 'pointer', gap: 0.5, '&:hover': { opacity: 0.8 } }}
      >
        {open
          ? <ExpandLessIcon sx={{ fontSize: 14, color: 'text.secondary' }} />
          : <ExpandMoreIcon sx={{ fontSize: 14, color: 'text.secondary' }} />}
        <Typography variant="caption" sx={{ ...mono, fontWeight: 600, color: 'text.secondary' }}>
          {label}
        </Typography>
      </Box>
      {open && (
        <Box sx={{ pl: 2.5, mt: 0.25 }}>
          <ContentBlock content={content} />
        </Box>
      )}
    </Box>
  )
}

// ── Content block with truncation ────────────────────────────────────────────

function ContentBlock({ content, maxLines = 20 }: { readonly content: string; readonly maxLines?: number }) {
  const [showAll, setShowAll] = useState(false)
  const lines = content.split('\n')
  const truncated = !showAll && lines.length > maxLines
  const display = truncated ? lines.slice(0, maxLines).join('\n') + '\n...' : content

  return (
    <Box>
      <Typography variant="caption" sx={{ ...mono, color: 'text.secondary', display: 'block' }}>
        {display}
      </Typography>
      {truncated && (
        <Typography
          variant="caption"
          onClick={() => setShowAll(true)}
          sx={{ ...mono, color: 'primary.main', cursor: 'pointer', '&:hover': { textDecoration: 'underline' } }}
        >
          Show all {lines.length} lines
        </Typography>
      )}
    </Box>
  )
}

export { AgentTraceCard }
export type { AgentTraceCardProps }
