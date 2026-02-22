import { useEffect, useRef } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useStore, dispatchStore, workflowStore } from '@/stores'
import { api } from '@/api'
import { DispatchTraceView } from './DispatchTraceView'

type DispatchTabProps = {
  stepId: string
}

function DispatchTab({ stepId }: DispatchTabProps) {
  const workflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const entry = useStore(dispatchStore.store, dispatchStore.selectByStepId(stepId))
  const fetchedRef = useRef(false)

  // Hydrate from API if no entry in store (page refresh / late tab open)
  useEffect(() => {
    if (entry !== null || fetchedRef.current || !workflowId) return
    fetchedRef.current = true

    const hydrate = async () => {
      try {
        const resp = await api.workflows.getStepDispatchHistory(workflowId, stepId)
        dispatchStore.hydrateFromApi(resp)
      } catch {
        // No dispatch history — leave empty state
      }
    }

    void hydrate()
  }, [stepId, workflowId, entry])

  if (entry === null) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', p: 2 }}>
        <Typography variant="body2" sx={{ color: 'text.secondary', fontStyle: 'italic' }}>
          No dispatch activity yet.
        </Typography>
      </Box>
    )
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Instruction header */}
      {entry.instruction.length > 0 && (
        <Box sx={{ px: 1.5, py: 0.75, borderBottom: 1, borderColor: 'divider', flexShrink: 0 }}>
          <Typography sx={{ fontSize: 11, color: 'text.secondary', fontStyle: 'italic' }}>
            {entry.instruction}
          </Typography>
        </Box>
      )}

      <DispatchTraceView entry={entry} />
    </Box>
  )
}

export { DispatchTab }
export type { DispatchTabProps }
