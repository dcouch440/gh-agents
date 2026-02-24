import { useEffect } from 'react'
import { Excalidraw } from '@excalidraw/excalidraw'
import '@excalidraw/excalidraw/index.css'
import Box from '@mui/material/Box'
import { boardStore } from '@/stores'
import { useBoardTheme, useBoardSubmit } from './hooks'
import { SubmitBar } from './SubmitBar'

type BoardProps = {
  readonly workflowId: string
}

/**
 * Full-screen Excalidraw drawing surface with a floating submit bar.
 *
 * Excalidraw owns its own state internally — we never mirror it into a store.
 * On submit, `useBoardSubmit` reads elements via the imperative API and
 * dispatches them through `boardStore.submitBoard`, which POSTs to the
 * backend and selectively syncs Phase 0 results into `workflowStore`.
 *
 * Resets `boardStore` on unmount (workflow change / navigation).
 */
function Board({ workflowId }: BoardProps) {
  const excalidrawTheme = useBoardTheme()
  const { setExcalidrawApi, handleSubmit, isSubmitting, error, status } = useBoardSubmit(workflowId)

  useEffect(() => {
    return () => { boardStore.resetBoard() }
  }, [workflowId])

  return (
    <Box sx={{ width: '100%', height: '100%', position: 'relative' }}>
      <Excalidraw
        theme={excalidrawTheme}
        excalidrawAPI={setExcalidrawApi}
      />
      <SubmitBar
        onSubmit={handleSubmit}
        isSubmitting={isSubmitting}
        status={status}
        error={error}
      />
    </Box>
  )
}

export { Board }
export type { BoardProps }
