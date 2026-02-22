import { useEffect, useRef } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useStore, dispatchStore } from '@/stores'
import { api } from '@/api'
import { DispatchTraceView } from './DispatchTraceView'

type DispatchTabProps = {
  stepId: string
}

function DispatchTab({ stepId }: DispatchTabProps) {
  const entry = useStore(dispatchStore.store, dispatchStore.selectByStepId(stepId))
  const fetchedRef = useRef(false)

  // Hydrate from API if no entry in store (page refresh / late tab open)
  useEffect(() => {
    if (entry !== null || fetchedRef.current) return
    fetchedRef.current = true

    const hydrate = async () => {
      const resp = await api.dispatch.listForStep(stepId)
      if (resp.tasks.length === 0) return

      // Pick the most recent task
      const latest = resp.tasks[resp.tasks.length - 1]
      if (latest === undefined) return

      const traceResp = await api.dispatch.trace(latest.execution_id)
      dispatchStore.hydrateFromApi(traceResp)
    }

    void hydrate()
  }, [stepId, entry])

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
