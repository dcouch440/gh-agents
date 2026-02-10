import { useState, useCallback } from 'react'

type CollapsibleState = {
  open: boolean
  onToggle: () => void
}

const useCollapsible = (defaultOpen = true): CollapsibleState => {
  const [open, setOpen] = useState(defaultOpen)
  const onToggle = useCallback(() => {
    setOpen((prev) => !prev)
  }, [])
  return { open, onToggle }
}

export { useCollapsible }
export type { CollapsibleState }
