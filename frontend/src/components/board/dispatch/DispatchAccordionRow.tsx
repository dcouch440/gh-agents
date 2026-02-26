import { useState, useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Chip from '@mui/material/Chip'
import IconButton from '@mui/material/IconButton'
import ExpandMoreIcon from '@mui/icons-material/ExpandMore'
import ExpandLessIcon from '@mui/icons-material/ExpandLess'
import { DispatchTraceView } from '@/components/canvas/CanvasNode/tabs/dispatch/DispatchTraceView'
import type { DispatchEntry } from '@/stores/dispatchStore'
import { statusColor } from './utils'

type DispatchAccordionRowProps = {
  readonly stepName: string
  readonly instruction: string
  readonly entry: DispatchEntry | null
}

function DispatchAccordionRow({ stepName, instruction, entry }: DispatchAccordionRowProps) {
  const [expanded, setExpanded] = useState(false)
  const toggle = useCallback(() => setExpanded((v) => !v), [])

  const status = entry?.status ?? null
  const toolCount = entry !== null
    ? entry.trace.filter((e) => e.type === 'tool_start').length
    : 0

  return (
    <Box
      sx={{
        borderBottom: 1,
        borderColor: 'divider',
        '&:last-child': { borderBottom: 0 },
      }}
    >
      {/* Collapsed header */}
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
          {stepName}
        </Typography>

        {status !== null ? (
          <Chip
            label={status}
            size="small"
            color={statusColor(status)}
            variant="outlined"
            sx={{ height: 20, fontSize: 10, flexShrink: 0 }}
          />
        ) : (
          <Typography
            sx={{
              fontSize: 10,
              color: 'text.disabled',
              fontFamily: 'monospace',
              flexShrink: 0,
              '@keyframes pulse': {
                '0%, 100%': { opacity: 1 },
                '50%': { opacity: 0.4 },
              },
              animation: 'pulse 1.5s ease-in-out infinite',
            }}
          >
            waiting...
          </Typography>
        )}
      </Box>

      {/* Compact info line (always visible when collapsed) */}
      {!expanded && (
        <Box sx={{ px: 1.5, pb: 0.5, display: 'flex', gap: 1, alignItems: 'center' }}>
          <Typography
            sx={{
              fontSize: 10,
              color: 'text.disabled',
              fontStyle: 'italic',
              fontFamily: 'monospace',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              flex: 1,
              minWidth: 0,
            }}
          >
            {instruction.slice(0, 80)}{instruction.length > 80 ? '...' : ''}
          </Typography>
          {toolCount > 0 && (
            <Typography sx={{ fontSize: 10, color: 'text.disabled', fontFamily: 'monospace', flexShrink: 0 }}>
              {toolCount} tool(s)
            </Typography>
          )}
        </Box>
      )}

      {/* Expanded detail */}
      {expanded && entry !== null && (
        <Box sx={{ display: 'flex', flexDirection: 'column' }}>
          {instruction.length > 0 && (
            <Box sx={{ px: 1.5, py: 0.5, borderTop: 1, borderColor: 'divider' }}>
              <Typography sx={{ fontSize: 11, color: 'text.secondary', fontStyle: 'italic' }}>
                {instruction}
              </Typography>
            </Box>
          )}

          {entry.summary !== null && entry.status === 'completed' && (
            <Box sx={{ px: 1.5, py: 0.5 }}>
              <Typography sx={{ fontFamily: 'monospace', fontSize: 11, color: 'success.main' }}>
                {entry.summary}
              </Typography>
            </Box>
          )}

          <Box sx={{ maxHeight: '60vh', overflowY: 'auto', minHeight: 80 }}>
            <DispatchTraceView entry={entry} />
          </Box>
        </Box>
      )}

      {expanded && entry === null && (
        <Box sx={{ px: 1.5, py: 1.5, borderTop: 1, borderColor: 'divider' }}>
          <Typography
            sx={{
              fontFamily: 'monospace',
              fontSize: 11,
              color: 'text.disabled',
              '@keyframes pulse': {
                '0%, 100%': { opacity: 1 },
                '50%': { opacity: 0.4 },
              },
              animation: 'pulse 1.5s ease-in-out infinite',
            }}
          >
            Waiting for dispatch events...
          </Typography>
        </Box>
      )}
    </Box>
  )
}

export { DispatchAccordionRow }
export type { DispatchAccordionRowProps }
