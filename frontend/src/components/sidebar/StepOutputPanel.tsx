import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useStore, workflowStore, workflowExecutionStore, sidebarStore } from '@/stores'
import { TerminalBlock } from '@/components/primitives/terminal-renderer'

function StepOutputPanel() {
  const selectedStepId = useStore(sidebarStore.store, sidebarStore.selectSelectedStepId)
  const step = useStore(workflowStore.store, workflowStore.selectStepById(selectedStepId))
  const stepState = useStore(
    workflowExecutionStore.store,
    selectedStepId ? workflowExecutionStore.selectStepState(selectedStepId) : () => undefined,
  )

  if (!step) return null

  const output = stepState?.output ?? null
  const error = stepState?.error ?? null
  const stepName = step.name ?? step.description

  return (
    <Box
      sx={{
        borderTop: 1,
        borderColor: 'divider',
        display: 'flex',
        flexDirection: 'column',
        flex: 1,
        minHeight: 0,
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <Box
        sx={{
          px: 1.5,
          py: 0.75,
          minHeight: 32,
          backgroundColor: (theme) => theme.palette.custom.bgHeader,
          borderBottom: 1,
          borderColor: 'divider',
        }}
      >
        <Typography
          variant="body2"
          sx={{
            fontSize: 11,
            fontWeight: 600,
            color: 'text.secondary',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
          }}
        >
          {stepName}
        </Typography>
      </Box>

      {/* Output content */}
      <Box sx={{ flex: 1, overflow: 'auto', p: 1.5 }}>
        {error ? (
          <Box
            sx={{
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
                fontFamily: '"JetBrains Mono", monospace',
                fontSize: '0.75rem',
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-word',
              }}
            >
              {error}
            </Typography>
          </Box>
        ) : output ? (
          <TerminalBlock content={output} />
        ) : (
          <Typography variant="body2" sx={{ color: 'text.disabled', fontStyle: 'italic', fontSize: 12 }}>
            No output yet
          </Typography>
        )}
      </Box>
    </Box>
  )
}

export { StepOutputPanel }
