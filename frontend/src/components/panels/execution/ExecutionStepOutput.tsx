import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { TerminalBlock } from '@/components/primitives/terminal-renderer'

type ExecutionStepOutputProps = {
  output: string | null
  error: string | null
}

function ExecutionStepOutput({ output, error }: ExecutionStepOutputProps) {
  if (error) {
    return (
      <Box
        sx={{
          mx: 1,
          mb: 1,
          p: 1.5,
          borderRadius: 1,
          backgroundColor: 'rgba(248, 81, 73, 0.08)',
          border: '1px solid rgba(248, 81, 73, 0.2)',
        }}
      >
        <Typography
          variant="body2"
          sx={{
            color: '#f85149',
            fontFamily: 'monospace',
            fontSize: '0.8125rem',
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-word',
          }}
        >
          {error}
        </Typography>
      </Box>
    )
  }

  if (output) {
    return (
      <Box
        sx={{
          mx: 1,
          mb: 1,
          maxHeight: 300,
          overflow: 'auto',
          borderRadius: 1,
          border: '1px solid',
          borderColor: 'divider',
          p: 1.5,
        }}
      >
        <TerminalBlock content={output} />
      </Box>
    )
  }

  return (
    <Box sx={{ mx: 1, mb: 1, py: 1 }}>
      <Typography variant="body2" sx={{ color: 'text.secondary', fontStyle: 'italic' }}>
        No output yet
      </Typography>
    </Box>
  )
}

export { ExecutionStepOutput }
export type { ExecutionStepOutputProps }
