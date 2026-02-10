import { Box, Typography, Paper } from '@mui/material'
import { ExecutionStatusBadge } from './ExecutionStatusBadge'
import { TimeAgo } from './TimeAgo'
import type { AgentExecution } from '@/types/execution'

type ReviewCardProps = {
  execution: AgentExecution
  selected: boolean
  onSelect: (id: string) => void
}

const truncate = (text: string, maxLen: number): string => (text.length > maxLen ? `${text.slice(0, maxLen)}...` : text)

function ReviewCard({ execution, selected, onSelect }: ReviewCardProps) {
  const firstLine = execution.input.split('\n')[0] ?? 'Untitled'

  return (
    <Paper
      elevation={0}
      onClick={() => onSelect(execution.id)}
      sx={{
        p: 1.5,
        cursor: 'pointer',
        border: 2,
        borderColor: selected ? 'primary.main' : 'divider',
        borderRadius: 1,
        bgcolor: selected ? 'action.selected' : 'background.paper',
        '&:hover': { bgcolor: selected ? 'action.selected' : 'action.hover' },
        transition: 'border-color 0.15s, background-color 0.15s',
      }}
    >
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 0.5 }}>
        <Typography variant="subtitle2" sx={{ fontWeight: 600, flex: 1, mr: 1 }} noWrap>
          {truncate(firstLine, 60)}
        </Typography>
        <ExecutionStatusBadge status={execution.status} />
      </Box>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace', fontSize: '0.75rem' }} noWrap>
          {truncate(execution.input, 80)}
        </Typography>
        <TimeAgo timestamp={execution.started_at} />
      </Box>
    </Paper>
  )
}

export { ReviewCard }
export type { ReviewCardProps }
