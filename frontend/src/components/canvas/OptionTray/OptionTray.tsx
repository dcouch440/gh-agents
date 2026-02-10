import { useState, useCallback } from 'react'
import { useStore, workflowStore } from '@/stores'
import { TrayPanel } from './TrayPanel'
import { TrayToggle } from './TrayToggle'
import { RunButton } from './RunButton'
import { SaveDiscardGroup } from './SaveDiscardGroup'

function OptionTray() {
  const dirty = useStore(workflowStore.store, workflowStore.selectDirty)
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
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

  if (!activeWorkflowId) return null

  return (
    <>
      <TrayPanel visible={open} dirty={dirty}>
        <SaveDiscardGroup />
        <RunButton />
      </TrayPanel>
      <TrayToggle open={open} onClick={handleToggle} />
    </>
  )
}

export { OptionTray }
