import { useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import AutoAwesome from '@mui/icons-material/AutoAwesome'
import Check from '@mui/icons-material/Check'
import KeyboardArrowDownRounded from '@mui/icons-material/KeyboardArrowDownRounded'
import Collapse from '@mui/material/Collapse'
import { getToolLabel } from '@/components/chat/toolLabels'

type ToolCallCardProps = {
  toolName: string
  toolId: string
  input: Record<string, unknown>
  result: unknown
  status: 'running' | 'complete'
}

const summarizeInput = (input: Record<string, unknown>): string => {
  const keys = Object.keys(input)
  if (keys.length === 0) return ''

  const pairs = keys.map((k) => {
    const v = input[k]
    const val = typeof v === 'string' ? `"${v}"` : String(JSON.stringify(v))
    return `${k}=${val}`
  })

  return pairs.join(', ')
}

const formatResult = (result: unknown): string => {
  if (result === null || result === undefined) return 'null'
  if (typeof result === 'string') return result
  return JSON.stringify(result, null, 2)
}

function ToolCallCard({ toolName, toolId, input, result, status }: ToolCallCardProps) {
  const [expanded, setExpanded] = useState(false)
  const isRunning = status === 'running'
  const label = getToolLabel(toolName, status)
  const inputSummary = summarizeInput(input)
  const hasResult = result !== null && result !== undefined

  return (
    <Box
      data-testid={`tool-call-${toolId}`}
      sx={{
        borderLeft: 2,
        borderColor: isRunning ? 'primary.main' : 'success.main',
        borderRadius: '0 4px 4px 0',
        bgcolor: 'action.hover',
        my: 0.5,
        ...(isRunning && {
          animation: 'toolPulse 2s ease-in-out infinite',
          '@keyframes toolPulse': {
            '0%, 100%': { opacity: 1 },
            '50%': { opacity: 0.6 },
          },
        }),
      }}
    >
      {/* Header row */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 0.75,
          py: 0.5,
          px: 1,
          cursor: hasResult ? 'pointer' : 'default',
        }}
        onClick={hasResult ? () => setExpanded((prev) => !prev) : undefined}
      >
        {isRunning ? (
          <AutoAwesome sx={{ fontSize: 14, color: 'primary.main' }} />
        ) : (
          <Check sx={{ fontSize: 14, color: 'success.main' }} />
        )}

        <Typography
          sx={{ fontFamily: 'monospace', fontSize: 11, color: 'text.secondary', fontWeight: 500 }}
        >
          {label}
        </Typography>

        {hasResult && (
          <IconButton
            size="small"
            sx={{
              ml: 'auto',
              width: 20,
              height: 20,
              transition: 'transform 0.2s',
              transform: expanded ? 'rotate(180deg)' : 'rotate(0deg)',
            }}
          >
            <KeyboardArrowDownRounded sx={{ fontSize: 14, color: 'text.disabled' }} />
          </IconButton>
        )}
      </Box>

      {/* Input summary */}
      {inputSummary.length > 0 && (
        <Typography
          sx={{
            fontFamily: 'monospace',
            fontSize: 10,
            color: 'text.disabled',
            px: 1,
            pb: 0.5,
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-word',
          }}
        >
          {inputSummary}
        </Typography>
      )}

      {/* Collapsible output */}
      {hasResult && (
        <Collapse in={expanded}>
          <Box
            component="pre"
            sx={{
              m: 0,
              px: 1,
              py: 0.5,
              fontFamily: 'monospace',
              fontSize: 10,
              color: 'text.secondary',
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-word',
              overflowY: 'auto',
              borderTop: 1,
              borderColor: 'divider',
            }}
          >
            {formatResult(result)}
          </Box>
        </Collapse>
      )}
    </Box>
  )
}

export { ToolCallCard }
export type { ToolCallCardProps }
