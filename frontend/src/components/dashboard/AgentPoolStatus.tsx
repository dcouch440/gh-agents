import { Box, Typography } from '@mui/material'
import type { Agent, AgentPoolStats } from '@/types'

type AgentPoolStatusProps = {
  agents: Agent[]
  stats: AgentPoolStats
}

const STATUS_DOT: Record<string, string> = {
  idle: '\u25CB',
  working: '\u25CF',
  waiting_for_context: '\u27F3',
  waiting_for_approval: '\u25C6',
}

const AGENT_STATUS_COLOR: Record<string, string> = {
  idle: 'text.disabled',
  working: 'text.primary',
  waiting_for_context: 'warning.main',
  waiting_for_approval: 'warning.main',
}

const AGENT_DOT_COLOR: Record<string, string> = {
  idle: 'text.disabled',
  working: 'success.main',
  waiting_for_context: 'warning.main',
  waiting_for_approval: 'warning.main',
}

const buildBar = (active: number, max: number): string => {
  const filled = '#'.repeat(active)
  const empty = '-'.repeat(max - active)
  return `[${filled}${empty}]`
}

function AgentPoolStatus({ agents, stats }: AgentPoolStatusProps) {
  const busy = agents.filter((a) => a.status && a.status !== 'idle')
  const active = stats.total - stats.available

  return (
    <Box sx={{ fontSize: '0.75rem', lineHeight: 1.4 }}>
      <Box sx={{ display: 'flex', gap: 1, py: '1px' }}>
        <Typography
          component="span"
          sx={{
            fontSize: 'inherit',
            lineHeight: 'inherit',
            color: 'text.disabled',
            width: '4ch',
            flexShrink: 0,
          }}
        >
          AGENTS
        </Typography>
        <Typography
          component="span"
          sx={{
            fontSize: 'inherit',
            lineHeight: 'inherit',
            color: 'text.disabled',
          }}
        >
          <Typography
            component="span"
            sx={{
              fontSize: 'inherit',
              lineHeight: 'inherit',
              color: 'success.main',
            }}
          >
            {buildBar(active, stats.max)}
          </Typography>
        </Typography>
        <Typography
          component="span"
          sx={{
            fontSize: 'inherit',
            lineHeight: 'inherit',
            color: 'text.secondary',
            fontVariantNumeric: 'tabular-nums',
          }}
        >
          {active}/{stats.max}
        </Typography>
      </Box>

      {busy.length > 0 ? (
        <Box sx={{ mt: '2px', display: 'flex', flexDirection: 'column', gap: 0 }}>
          {busy.map((a) => (
            <Box
              key={a.id}
              sx={{
                py: '1px',
                color: AGENT_STATUS_COLOR[a.status] ?? 'text.disabled',
              }}
            >
              <Typography
                component="span"
                sx={{
                  fontSize: 'inherit',
                  lineHeight: 'inherit',
                  display: 'inline',
                  color: AGENT_DOT_COLOR[a.status] ?? 'text.disabled',
                }}
              >
                {STATUS_DOT[a.status] ?? '\u25CB'}
              </Typography>{' '}
              {a.name}
            </Box>
          ))}
        </Box>
      ) : null}
    </Box>
  )
}

export { AgentPoolStatus }
export type { AgentPoolStatusProps }
