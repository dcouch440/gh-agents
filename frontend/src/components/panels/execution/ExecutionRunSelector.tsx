import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import AccessTimeOutlined from '@mui/icons-material/AccessTimeOutlined'

type ExecutionRunSelectorProps = {
  currentRunId: string | null
}

function ExecutionRunSelector({ currentRunId }: ExecutionRunSelectorProps) {
  if (!currentRunId) return null

  const truncated = currentRunId.slice(0, 8)

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 0.75,
        px: 2,
        py: 0.75,
        borderBottom: 1,
        borderColor: 'divider',
      }}
    >
      <AccessTimeOutlined sx={{ fontSize: 14, color: 'text.secondary' }} />
      <Typography variant="caption" sx={{ color: 'text.secondary', fontFamily: 'monospace' }}>
        Run {truncated}
      </Typography>
    </Box>
  )
}

export { ExecutionRunSelector }
export type { ExecutionRunSelectorProps }
