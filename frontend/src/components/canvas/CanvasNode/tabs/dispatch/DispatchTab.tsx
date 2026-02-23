import { useEffect, useRef } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import CircularProgress from '@mui/material/CircularProgress'
import { useStore, dispatchStore, dispatchSessionStore } from '@/stores'
import { MessageList } from '@/components/chat'
import { DispatchTraceView } from './DispatchTraceView'

type DispatchTabProps = {
  stepId: string
}

function DispatchTab({ stepId }: DispatchTabProps) {
  const messages = useStore(dispatchSessionStore.store, dispatchSessionStore.selectMessages(stepId))
  const isLoading = useStore(dispatchSessionStore.store, dispatchSessionStore.selectLoading(stepId))
  const error = useStore(dispatchSessionStore.store, dispatchSessionStore.selectError(stepId))
  const activeEntry = useStore(dispatchStore.store, dispatchStore.selectActiveForStep(stepId))
  const completedEntry = useStore(dispatchStore.store, dispatchStore.selectByStepId(stepId))
  const loadedRef = useRef(false)

  // Load session history on mount
  useEffect(() => {
    if (loadedRef.current) return
    loadedRef.current = true
    void dispatchSessionStore.loadSession(stepId)
    return () => {
      loadedRef.current = false
    }
  }, [stepId])

  // When a dispatch completes, append to session store and reload
  const prevStatusRef = useRef<string | null>(null)
  useEffect(() => {
    if (!completedEntry) {
      prevStatusRef.current = null
      return
    }
    const wasRunning = prevStatusRef.current === 'running'
    prevStatusRef.current = completedEntry.status

    if (wasRunning && completedEntry.status === 'completed' && completedEntry.summary) {
      dispatchSessionStore.appendDispatchResult(
        stepId,
        completedEntry.instruction,
        completedEntry.summary,
      )
    }
  }, [completedEntry, stepId])

  if (isLoading) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <CircularProgress size={20} />
      </Box>
    )
  }

  if (error && messages.length === 0) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', p: 2 }}>
        <Typography variant="body2" color="error">
          {error}
        </Typography>
      </Box>
    )
  }

  const hasHistory = messages.length > 0
  const hasActiveTrace = activeEntry !== null

  if (!hasHistory && !hasActiveTrace) {
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
      {/* Historical dispatch messages */}
      {hasHistory && (
        <Box sx={{ flex: hasActiveTrace ? undefined : 1, minHeight: 0, overflowY: hasActiveTrace ? undefined : 'auto' }}>
          <MessageList
            messages={messages}
            emptyMessage="No dispatch activity yet."
          />
        </Box>
      )}

      {/* Active dispatch trace */}
      {hasActiveTrace && (
        <Box sx={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', borderTop: hasHistory ? 1 : 0, borderColor: 'divider' }}>
          {activeEntry.instruction.length > 0 && (
            <Box sx={{ px: 1.5, py: 0.75, borderBottom: 1, borderColor: 'divider', flexShrink: 0 }}>
              <Typography sx={{ fontSize: 11, color: 'text.secondary', fontStyle: 'italic' }}>
                {activeEntry.instruction}
              </Typography>
            </Box>
          )}
          <DispatchTraceView entry={activeEntry} />
        </Box>
      )}
    </Box>
  )
}

export { DispatchTab }
export type { DispatchTabProps }
