import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useStore, workflowStore } from '@/stores'
import { TerminalBlock } from '@/components/primitives/terminal-renderer'

type PlanTabProps = {
  stepId: string
}

function PlanTab({ stepId }: PlanTabProps) {
  const planByStep = useStore(workflowStore.store, workflowStore.selectPlanByStep)
  const content = planByStep[stepId] ?? ''
  const isEmpty = !content.trim()

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Box sx={{ flex: 1, overflow: 'hidden', pt: 0.5, px: 0.5, pb: 0.5 }}>
        {isEmpty ? (
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              height: '100%',
            }}
          >
            <Typography sx={{ fontSize: 12, color: 'text.disabled', fontStyle: 'italic' }}>
              Plan will appear as the assistant records it.
            </Typography>
          </Box>
        ) : (
          <Box sx={{ px: 1, py: 0.5, overflow: 'auto', height: '100%' }}>
            <TerminalBlock content={content} />
          </Box>
        )}
      </Box>
    </Box>
  )
}

export { PlanTab }
