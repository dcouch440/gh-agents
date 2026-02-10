import { Box, Typography } from '@mui/material'
import type { Config, AgentPoolStats } from '@/types'

type SystemHealthStatusProps = {
  config: Config
  agentStats: AgentPoolStats
  wsConnected: boolean
}

function SystemHealthStatus({ config, agentStats, wsConnected }: SystemHealthStatusProps) {
  // AgentPoolStats structure is defined in types/agent.ts
  const poolStr = `${agentStats.total - agentStats.available}/${agentStats.max} agents`

  return (
    <Box sx={{ fontSize: '0.75rem', lineHeight: 1.4 }}>
      <Box sx={{ display: 'flex', gap: 1, py: '1px' }}>
        <Typography
          component="span"
          sx={{ fontSize: 'inherit', lineHeight: 'inherit', color: 'text.disabled', width: '10ch', flexShrink: 0 }}
        >
          MODE
        </Typography>
        <Typography component="span" sx={{ fontSize: 'inherit', lineHeight: 'inherit', color: 'text.primary' }}>
          {config.autonomy}
        </Typography>
      </Box>
      <Box sx={{ display: 'flex', gap: 1, py: '1px' }}>
        <Typography
          component="span"
          sx={{ fontSize: 'inherit', lineHeight: 'inherit', color: 'text.disabled', width: '10ch', flexShrink: 0 }}
        >
          GIT
        </Typography>
        <Typography component="span" sx={{ fontSize: 'inherit', lineHeight: 'inherit', color: 'text.primary' }}>
          {config.git_strategy}
        </Typography>
      </Box>
      <Box sx={{ display: 'flex', gap: 1, py: '1px' }}>
        <Typography
          component="span"
          sx={{ fontSize: 'inherit', lineHeight: 'inherit', color: 'text.disabled', width: '10ch', flexShrink: 0 }}
        >
          VERBOSITY
        </Typography>
        <Typography component="span" sx={{ fontSize: 'inherit', lineHeight: 'inherit', color: 'text.primary' }}>
          {config.verbosity}
        </Typography>
      </Box>
      <Box sx={{ display: 'flex', gap: 1, py: '1px' }}>
        <Typography
          component="span"
          sx={{ fontSize: 'inherit', lineHeight: 'inherit', color: 'text.disabled', width: '10ch', flexShrink: 0 }}
        >
          SANDBOX
        </Typography>
        <Typography component="span" sx={{ fontSize: 'inherit', lineHeight: 'inherit', color: 'text.primary' }}>
          {config.sandbox_mode}
        </Typography>
      </Box>
      <Box sx={{ display: 'flex', gap: 1, py: '1px' }}>
        <Typography
          component="span"
          sx={{ fontSize: 'inherit', lineHeight: 'inherit', color: 'text.disabled', width: '10ch', flexShrink: 0 }}
        >
          WS
        </Typography>
        <Typography
          component="span"
          sx={{
            fontSize: 'inherit',
            lineHeight: 'inherit',
            color: wsConnected ? 'success.main' : 'error.main',
            ...(wsConnected
              ? {}
              : {
                  animation: 'blink 800ms step-end infinite',
                  '@keyframes blink': {
                    '0%, 100%': { opacity: 1 },
                    '50%': { opacity: 0 },
                  },
                }),
          }}
        >
          {wsConnected ? 'connected' : 'disconnected'}
        </Typography>
      </Box>
      <Box sx={{ display: 'flex', gap: 1, py: '1px' }}>
        <Typography
          component="span"
          sx={{ fontSize: 'inherit', lineHeight: 'inherit', color: 'text.disabled', width: '10ch', flexShrink: 0 }}
        >
          POOL
        </Typography>
        <Typography component="span" sx={{ fontSize: 'inherit', lineHeight: 'inherit', color: 'text.primary' }}>
          {poolStr}
        </Typography>
      </Box>
    </Box>
  )
}

export { SystemHealthStatus }
export type { SystemHealthStatusProps }
