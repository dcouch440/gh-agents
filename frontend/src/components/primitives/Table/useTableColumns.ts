import {useState, useMemo, useCallback} from 'react'
import type {TableColumn} from './types'

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

function useTableColumns<T>({
  columns,
  defaultVisibleColumns,
}: UseTableColumnsProps<T>): UseTableColumnsReturn<T> {
  // Initialize visible columns
  const [hiddenColumnKeys, setHiddenColumnKeys] = useState<Set<string>>(() => {
    if (defaultVisibleColumns) {
      const defaultSet = new Set(defaultVisibleColumns)
      return new Set(columns.filter((col) => !defaultSet.has(col.key)).map((col) => col.key))
    }
    return new Set()
  })

  // Filter visible columns
  const visibleColumns = useMemo(
    () => columns.filter((col) => !hiddenColumnKeys.has(col.key)),
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
      const allKeysExceptFirst = columns.slice(1).map((col) => col.key)
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

export {useTableColumns}
export type {UseTableColumnsProps, UseTableColumnsReturn}
