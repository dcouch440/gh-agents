import { useState, useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import ExpandMoreIcon from '@mui/icons-material/ExpandMore'
import ExpandLessIcon from '@mui/icons-material/ExpandLess'
import { useStore } from '@/stores/lib'
import { agentTraceStore } from '@/stores/agentTraceStore'
import type { AgentTrace, AgentTraceEvent } from '@/stores/agentTraceStore'

// ── Collapsible section ──────────────────────────────────────────────────────

type CollapsibleProps = {
  readonly label: string
  readonly children: React.ReactNode
  readonly defaultOpen?: boolean
  readonly color?: string
}

function Collapsible({ label, children, defaultOpen = false, color = 'text.secondary' }: CollapsibleProps) {
  const [open, setOpen] = useState(defaultOpen)
  return (
    <Box sx={{ mb: 0.5 }}>
      <Box
        onClick={() => setOpen((v) => !v)}
        sx={{ display: 'flex', alignItems: 'center', cursor: 'pointer', gap: 0.5, '&:hover': { opacity: 0.8 } }}
      >
        {open
          ? <ExpandLessIcon sx={{ fontSize: 14, color }} />
          : <ExpandMoreIcon sx={{ fontSize: 14, color }} />}
        <Typography variant="caption" sx={{ fontFamily: 'monospace', fontSize: 11, fontWeight: 600, color }}>
          {label}
        </Typography>
      </Box>
      {open && (
        <Box sx={{ pl: 2.5, mt: 0.25 }}>
          {children}
        </Box>
      )}
    </Box>
  )
}

// ── Event renderers ──────────────────────────────────────────────────────────

const mono = { fontFamily: 'monospace', fontSize: 11, lineHeight: 1.5, whiteSpace: 'pre-wrap', wordBreak: 'break-word' } as const

function ContentBlock({ content, maxLines = 20 }: { readonly content: string; readonly maxLines?: number }) {
  const [expanded, setExpanded] = useState(false)
  const lines = content.split('\n')
  const truncated = !expanded && lines.length > maxLines
  const display = truncated ? lines.slice(0, maxLines).join('\n') + '\n...' : content

  return (
    <Box>
      <Typography variant="caption" sx={{ ...mono, color: 'text.secondary', display: 'block' }}>
        {display}
      </Typography>
      {truncated && (
        <Typography
          variant="caption"
          onClick={() => setExpanded(true)}
          sx={{ ...mono, color: 'primary.main', cursor: 'pointer', '&:hover': { textDecoration: 'underline' } }}
        >
          Show all {lines.length} lines
        </Typography>
      )}
    </Box>
  )
}

function ToolCallEvent({ event }: { readonly event: AgentTraceEvent & { type: 'tool_call' } }) {
  return (
    <Collapsible label={`tool: ${event.toolName}`} color="info.main">
      <Typography variant="caption" component="pre" sx={{ ...mono, color: 'text.secondary' }}>
        {JSON.stringify(event.input, null, 2)}
      </Typography>
    </Collapsible>
  )
}

function ToolResultEvent({ event }: { readonly event: AgentTraceEvent & { type: 'tool_result' } }) {
  return (
    <Collapsible label={`result: ${event.toolName}`} color="success.main">
      <ContentBlock content={event.result} maxLines={10} />
    </Collapsible>
  )
}

// ── Single agent trace ───────────────────────────────────────────────────────

function AgentTraceCard({ trace }: { readonly trace: AgentTrace }) {
  const [expanded, setExpanded] = useState(false)
  const toggle = useCallback(() => setExpanded((v) => !v), [])

  const systemPrompt = trace.events.find((e) => e.type === 'system_prompt')
  const userMessage = trace.events.find((e) => e.type === 'user_message')
  const assistantMessage = trace.events.find((e) => e.type === 'assistant_message')
  const toolEvents = trace.events.filter((e) => e.type === 'tool_call' || e.type === 'tool_result')

  const displayName = trace.agentName ?? 'Agent'

  return (
    <Box
      sx={{
        border: 1,
        borderColor: 'divider',
        borderRadius: 1,
        mb: 0.75,
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <Box
        onClick={toggle}
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          px: 1,
          py: 0.5,
          cursor: 'pointer',
          bgcolor: 'action.hover',
          '&:hover': { bgcolor: 'action.selected' },
        }}
      >
        <Typography variant="caption" sx={{ fontFamily: 'monospace', fontSize: 11, fontWeight: 700 }}>
          {displayName}
        </Typography>
        <IconButton size="small" sx={{ p: 0 }}>
          {expanded
            ? <ExpandLessIcon sx={{ fontSize: 14 }} />
            : <ExpandMoreIcon sx={{ fontSize: 14 }} />}
        </IconButton>
      </Box>

      {/* Expanded content */}
      {expanded && (
        <Box sx={{ px: 1, py: 0.5 }}>
          {systemPrompt?.type === 'system_prompt' && (
            <Collapsible label="System Prompt">
              <ContentBlock content={systemPrompt.content} />
            </Collapsible>
          )}

          {userMessage?.type === 'user_message' && (
            <Collapsible label="Input">
              <ContentBlock content={userMessage.content} />
            </Collapsible>
          )}

          {toolEvents.length > 0 && (
            <Collapsible label={`Tools (${toolEvents.length})`} defaultOpen>
              {toolEvents.map((event, i) =>
                event.type === 'tool_call'
                  ? <ToolCallEvent key={`${event.toolId}-call-${i}`} event={event} />
                  : <ToolResultEvent key={`${event.toolId}-result-${i}`} event={event} />
              )}
            </Collapsible>
          )}

          {assistantMessage?.type === 'assistant_message' && (
            <Collapsible label="Output" defaultOpen>
              <ContentBlock content={assistantMessage.content} />
            </Collapsible>
          )}
        </Box>
      )}
    </Box>
  )
}

// ── Panel ────────────────────────────────────────────────────────────────────

function AgentTracePanel() {
  const traces = useStore(agentTraceStore.store, agentTraceStore.selectTraces)
  const order = useStore(agentTraceStore.store, agentTraceStore.selectOrder)

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', minHeight: 0 }}>
      <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', mb: 0.5, px: 0.5 }}>
        Agent Execution Trace
      </Typography>
      {order.length === 0 && (
        <Typography variant="caption" sx={{ color: 'text.disabled', fontFamily: 'monospace', fontSize: 11, px: 0.5 }}>
          No agent traces yet
        </Typography>
      )}
      {order.map((id) => {
        const trace = traces[id]
        if (trace === undefined) return null
        return <AgentTraceCard key={id} trace={trace} />
      })}
    </Box>
  )
}

export { AgentTracePanel }
