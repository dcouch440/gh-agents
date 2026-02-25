import { useEffect, useMemo, useState } from 'react'
import { Excalidraw } from '@excalidraw/excalidraw'
import '@excalidraw/excalidraw/index.css'
import Box from '@mui/material/Box'
import CircularProgress from '@mui/material/CircularProgress'
import { boardStore } from '@/stores'
import { useBoardTheme, useBoardSubmit, useBoardElements, useDispatchHistory } from './hooks'
import { SubmitBar } from './SubmitBar'
import { DebugPanel } from './debug'

type BoardProps = {
  readonly workflowId: string
}

/**
 * Full-screen Excalidraw drawing surface with a floating submit bar.
 *
 * Loads saved elements from the backend on mount so the user sees their
 * previous drawings. Excalidraw owns its state internally — we only
 * read from it on submit via the imperative API.
 *
 * Resets `boardStore` on unmount (workflow change / navigation).
 */
function Board({ workflowId }: BoardProps) {
  const excalidrawTheme = useBoardTheme()
  const { setExcalidrawApi, handleSubmit, isSubmitting, error, status } = useBoardSubmit(workflowId)
  const { loading, elements: savedElements } = useBoardElements(workflowId)
  const [showDebug, setShowDebug] = useState(false)

  // Fetch historical dispatch traces for existing steps
  useDispatchHistory(workflowId)

  useEffect(() => {
    return () => { boardStore.resetBoard() }
  }, [workflowId])

  // Excalidraw's initialData only applies on first render, so we must
  // wait for the fetch to complete before mounting it.
  const initialData = useMemo(
    () => (savedElements !== null ? { elements: savedElements } : undefined),
    [savedElements],
  )

  return (
    <Box sx={{ width: '100%', height: '100%', position: 'relative' }}>
      {loading ? (
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
          <CircularProgress size={32} />
        </Box>
      ) : (
        <Excalidraw
          theme={excalidrawTheme}
          excalidrawAPI={setExcalidrawApi}
          initialData={initialData}
        />
      )}
      <SubmitBar
        onSubmit={handleSubmit}
        isSubmitting={isSubmitting}
        status={status}
        error={error}
        showDebug={showDebug}
        onToggleDebug={() => setShowDebug((v) => !v)}
      />
      {showDebug && <DebugPanel onClose={() => setShowDebug(false)} />}
    </Box>
  )
}

export { Board }
export type { BoardProps }
