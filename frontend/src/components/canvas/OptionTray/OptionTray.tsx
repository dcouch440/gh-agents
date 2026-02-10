import { useState, useCallback } from 'react'
import { useStore, workflowStore } from '@/stores'
import { TrayPanel } from './TrayPanel'
import { TrayToggle } from './TrayToggle'
import { RunButton } from './RunButton'
import { SaveDiscardGroup } from './SaveDiscardGroup'

function OptionTray() {
  const dirty = useStore(workflowStore.store, workflowStore.selectDirty)
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const [manualOpen, setManualOpen] = useState(false)
  const [prevDirty, setPrevDirty] = useState(dirty)

  // Auto-open tray when dirty, auto-dismiss when dirty clears
  if (prevDirty !== dirty) {
    setPrevDirty(dirty)
    if (dirty) setManualOpen(true)
    else setManualOpen(false)
  }

  const handleToggle = useCallback(() => {
    setManualOpen((prev) => !prev)
  }, [])

  if (!activeWorkflowId) return null

  return (
    <>
      <TrayPanel visible={manualOpen} dirty={dirty}>
        <SaveDiscardGroup />
        <RunButton />
      </TrayPanel>
      <TrayToggle open={manualOpen} onClick={handleToggle} />
    </>
  )
}

export { OptionTray }
