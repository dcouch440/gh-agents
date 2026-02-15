import { useEffect, useRef, useState, useCallback } from 'react'
import { useStore, workflowStore } from '@/stores'
import { useDebounceCallback } from './useDebounceCallback'
import { AUTO_SAVE_DEBOUNCE_MS } from '@/constants'

type AutoSaveHandle = {
  /** Force-save all dirty steps immediately. */
  flush: () => void
  /** True while a save API call is in flight. */
  saving: boolean
}

/**
 * Watches the workflow store's dirty flag and auto-saves after a debounce.
 * Flushes pending saves on unmount to prevent data loss on navigation.
 */
const useAutoSave = (enabled: boolean): AutoSaveHandle => {
  const dirty = useStore(workflowStore.store, workflowStore.selectDirty)
  const [saving, setSaving] = useState(false)
  const savingRef = useRef(false)

  const save = useCallback(async () => {
    if (savingRef.current) return
    savingRef.current = true
    setSaving(true)
    try {
      await workflowStore.saveAllDirtySteps()
    } finally {
      savingRef.current = false
      setSaving(false)
    }
  }, [])

  const debounced = useDebounceCallback(
    () => {
      void save()
    },
    AUTO_SAVE_DEBOUNCE_MS,
    { flushOnUnmount: true },
  )

  // Trigger debounced save whenever dirty becomes true
  const prevDirtyRef = useRef(dirty)
  useEffect(() => {
    if (!enabled) return
    // Fire on any transition to dirty, or when dirty set changes (new edits while already dirty)
    if (dirty && !prevDirtyRef.current) {
      debounced.call(undefined)
    } else if (dirty && prevDirtyRef.current) {
      // Still dirty — reset the debounce timer (user is still editing)
      debounced.call(undefined)
    }
    prevDirtyRef.current = dirty
  }, [dirty, enabled, debounced])

  const flush = useCallback(() => {
    debounced.flush()
  }, [debounced])

  return { flush, saving }
}

export { useAutoSave }
