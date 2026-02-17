import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import CircularProgress from '@mui/material/CircularProgress'
import CheckCircleOutlined from '@mui/icons-material/CheckCircleOutlined'

import type { StreamToolUse } from '@/stores/stepStreamStore'

type ToolActivityFeedProps = {
  tools: StreamToolUse[]
  compact?: boolean
}

function ToolActivityFeed({ tools, compact = false }: ToolActivityFeedProps) {
  if (tools.length === 0) return null

  if (compact) {
    return (
      <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
        {tools.map((tool) => (
          <Box
            key={tool.toolId}
            sx={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 0.5,
              px: 0.75,
              py: 0.25,
              borderRadius: '100px',
              backgroundColor: 'action.hover',
            }}
          >
            <Box
              sx={{
                width: 4,
                height: 4,
                borderRadius: '50%',
                backgroundColor:
                  tool.status === 'running' ? '#f59e0b' : '#10b981',
                flexShrink: 0,
              }}
            />
            <Typography sx={{ fontSize: 9, color: 'text.secondary' }}>
              {tool.toolName}
            </Typography>
          </Box>
        ))}
      </Box>
    )
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
      {tools.map((tool) => (
        <Box
          key={tool.toolId}
          sx={{ display: 'flex', alignItems: 'center', gap: 0.75 }}
        >
          {tool.status === 'running' ? (
            <CircularProgress size={12} />
          ) : (
            <CheckCircleOutlined
              sx={{ fontSize: 12, color: '#10b981' }}
            />
          )}
          <Typography sx={{ fontSize: 11, color: 'text.secondary' }}>
            {tool.toolName}
          </Typography>
        </Box>
      ))}
    </Box>
  )
}

export { ToolActivityFeed }
export type { ToolActivityFeedProps }
