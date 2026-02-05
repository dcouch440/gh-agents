import { Box } from '@mui/material'
import { EmptyState, Button } from '@/components/primitives'

type NoRouterStateProps = {
  onCreateRouter: () => void
  creating: boolean
}

function NoRouterState({ onCreateRouter, creating }: NoRouterStateProps) {
  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2 }}>
      <EmptyState message="No tool router configured for this agent" />
      <Button variant="primary" onClick={onCreateRouter} disabled={creating}>
        {creating ? 'Creating...' : 'Create Router'}
      </Button>
    </Box>
  )
}

export { NoRouterState }
export type { NoRouterStateProps }
