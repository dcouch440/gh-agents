import { useState, useCallback } from 'react'
import { useStore, workflowStore, canvasStore, focusModeStore } from '@/stores'
import { TrayPanel } from './TrayPanel'
import { TrayToggle } from './TrayToggle'
import { RunButton } from './RunButton'
import { SaveDiscardGroup } from './SaveDiscardGroup'
import { FocusModeButton } from './FocusModeButton'
import { topoSortStepIds } from '@/utils/topoSort'

type OptionTrayProps = {
  autoSaveFlush: () => void
  autoSaveSaving: boolean
}

function OptionTray({ autoSaveFlush, autoSaveSaving }: OptionTrayProps) {
  const dirty = useStore(workflowStore.store, workflowStore.selectDirty)
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const edges = useStore(workflowStore.store, workflowStore.selectEdges)
  const [open, setOpen] = useState(false)
  const [lockedOpen, setLockedOpen] = useState(false)
  const [prevDirty, setPrevDirty] = useState(dirty)

  // Auto-open tray when dirty, auto-close only if user hasn't locked it open
  if (prevDirty !== dirty) {
    setPrevDirty(dirty)
    if (dirty) setOpen(true)
    else if (!lockedOpen) setOpen(false)
  }

  const handleToggle = useCallback(() => {
    setOpen((prev) => {
      const next = !prev
      setLockedOpen(next)
      return next
    })
  }, [])

  const handleEnterFocusMode = useCallback(() => {
    const ordered = topoSortStepIds(steps, edges, { includeAll: true })
    if (ordered.length === 0) return
    const selectedIds = canvasStore.store.getState().selectedStepIds
    const initialId = ordered.find((id) => selectedIds.has(id))
    focusModeStore.enter(ordered, initialId)
  }, [steps, edges])

  if (!activeWorkflowId) return null

  return (
    <>
      <TrayPanel visible={open} dirty={dirty}>
        <SaveDiscardGroup autoSaveFlush={autoSaveFlush} autoSaveSaving={autoSaveSaving} />
        <RunButton />
        <FocusModeButton onClick={handleEnterFocusMode} />
      </TrayPanel>
      <TrayToggle open={open} onClick={handleToggle} />
    </>
  )
}

export { OptionTray }
