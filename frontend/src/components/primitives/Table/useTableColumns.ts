import { useState, useMemo, useCallback } from 'react'
import { Collections } from '@/utils/collections'
import type { TableColumn } from './types'

type UseTableColumnsProps<T> = {
  columns: TableColumn<T>[]
  defaultVisibleColumns?: string[]
}

type UseTableColumnsReturn<T> = {
  visibleColumns: TableColumn<T>[]
  hiddenColumnKeys: Set<string>
  toggleColumnVisibility: (columnKey: string) => void
  showAllColumns: () => void
  hideAllColumns: () => void
}

function useTableColumns<T>({ columns, defaultVisibleColumns }: UseTableColumnsProps<T>): UseTableColumnsReturn<T> {
  // Initialize visible columns
  const [hiddenColumnKeys, setHiddenColumnKeys] = useState<Set<string>>(() => {
    if (defaultVisibleColumns) {
      const defaultSet = new Set(defaultVisibleColumns)
      return new Set(Collections.filterMap(columns, (col) => (!defaultSet.has(col.key) ? col.key : null)))
    }
    return new Set()
  })

  // Filter visible columns
  const visibleColumns = useMemo(
    () => Collections.filterMap(columns, (col) => (!hiddenColumnKeys.has(col.key) ? col : null)),
    [columns, hiddenColumnKeys],
  )

  // Toggle column visibility
  const toggleColumnVisibility = useCallback((columnKey: string) => {
    setHiddenColumnKeys((prev) => {
      const next = new Set(prev)
      if (next.has(columnKey)) {
        next.delete(columnKey)
      } else {
        next.add(columnKey)
      }
      return next
    })
  }, [])

  // Show all columns
  const showAllColumns = useCallback(() => {
    setHiddenColumnKeys(new Set())
  }, [])

  // Hide all columns (except at least one must remain visible)
  const hideAllColumns = useCallback(() => {
    if (columns.length > 0) {
      const allKeysExceptFirst = Collections.mapBy(columns.slice(1), (col) => col.key)
      setHiddenColumnKeys(new Set(allKeysExceptFirst))
    }
  }, [columns])

  return {
    visibleColumns,
    hiddenColumnKeys,
    toggleColumnVisibility,
    showAllColumns,
    hideAllColumns,
  }
}

export { useTableColumns }
export type { UseTableColumnsProps, UseTableColumnsReturn }
