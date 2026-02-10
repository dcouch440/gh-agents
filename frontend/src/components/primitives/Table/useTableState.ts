import { useState, useMemo, useCallback } from 'react'
import type { SortDirection, TableColumn } from './types'
import { sortData, filterData, paginateData } from './utils'

type UseTableStateProps<T> = {
  data: T[]
  columns: TableColumn<T>[]
  defaultSortColumn?: string
  defaultSortDirection?: SortDirection
  defaultPageSize?: number
  searchFields?: (keyof T)[]
  enableSorting?: boolean
  enableSearch?: boolean
  enablePagination?: boolean
}

type UseTableStateReturn<T> = {
  // Processed data
  displayedData: T[]
  totalRows: number
  filteredRows: number

  // Sort state
  sortColumn: string | null
  sortDirection: SortDirection
  handleSort: (column: string) => void

  // Search state
  searchQuery: string
  setSearchQuery: (query: string) => void

  // Pagination state
  page: number
  pageSize: number
  setPage: (page: number) => void
  setPageSize: (size: number) => void
  totalPages: number

  // Selection state
  selectedRowKeys: Set<string>
  toggleRowSelection: (key: string) => void
  toggleAllSelection: (allKeys: string[]) => void
  clearSelection: () => void
}

function useTableState<T>({
  data,
  columns,
  defaultSortColumn,
  defaultSortDirection = 'asc',
  defaultPageSize = 25,
  searchFields,
  enableSorting = false,
  enableSearch = false,
  enablePagination = false,
}: UseTableStateProps<T>): UseTableStateReturn<T> {
  // Sort state
  const [sortColumn, setSortColumn] = useState<string | null>(defaultSortColumn ?? null)
  const [sortDirection, setSortDirection] = useState<SortDirection>(defaultSortDirection)

  // Search state
  const [searchQuery, setSearchQuery] = useState('')

  // Pagination state
  const [page, setPage] = useState(0)
  const [pageSize, setPageSize] = useState(defaultPageSize)

  // Selection state
  const [selectedRowKeys, setSelectedRowKeys] = useState<Set<string>>(new Set())

  // Handle sort toggle
  const handleSort = useCallback(
    (column: string) => {
      if (!enableSorting) return

      if (sortColumn === column) {
        // Toggle direction: asc -> desc -> null
        if (sortDirection === 'asc') {
          setSortDirection('desc')
        } else {
          setSortColumn(null)
          setSortDirection('asc')
        }
      } else {
        setSortColumn(column)
        setSortDirection('asc')
      }

      // Reset to first page on sort
      setPage(0)
    },
    [sortColumn, sortDirection, enableSorting],
  )

  // Process data: filter -> sort -> paginate
  const processedData = useMemo(() => {
    let result = data

    // 1. Filter (search)
    if (enableSearch && searchQuery) {
      result = filterData(result, searchQuery, searchFields)
    }

    const filteredCount = result.length

    // 2. Sort
    if (enableSorting && sortColumn) {
      const column = columns.find((col) => col.key === sortColumn)
      result = sortData(result, sortColumn, sortDirection, column?.sortFn)
    }

    // 3. Paginate
    let paginatedResult = result
    if (enablePagination) {
      paginatedResult = paginateData(result, page, pageSize)
    }

    return {
      displayed: paginatedResult,
      totalRows: data.length,
      filteredRows: filteredCount,
    }
  }, [data, columns, sortColumn, sortDirection, searchQuery, searchFields, page, pageSize, enableSorting, enableSearch, enablePagination])

  // Calculate total pages
  const totalPages = useMemo(() => {
    if (!enablePagination) return 1
    return Math.ceil(processedData.filteredRows / pageSize)
  }, [processedData.filteredRows, pageSize, enablePagination])

  // Update page when it exceeds total pages
  if (page >= totalPages && totalPages > 0) {
    setPage(Math.max(0, totalPages - 1))
  }

  // Selection handlers
  const toggleRowSelection = useCallback((key: string) => {
    setSelectedRowKeys((prev) => {
      const next = new Set(prev)
      if (next.has(key)) {
        next.delete(key)
      } else {
        next.add(key)
      }
      return next
    })
  }, [])

  const toggleAllSelection = useCallback((allKeys: string[]) => {
    setSelectedRowKeys((prev) => {
      // If all are selected, deselect all
      if (allKeys.every((key) => prev.has(key))) {
        return new Set()
      }
      // Otherwise, select all
      return new Set(allKeys)
    })
  }, [])

  const clearSelection = useCallback(() => {
    setSelectedRowKeys(new Set())
  }, [])

  // Update search query and reset to first page
  const handleSetSearchQuery = useCallback((query: string) => {
    setSearchQuery(query)
    setPage(0)
  }, [])

  // Update page size and reset to first page
  const handleSetPageSize = useCallback((size: number) => {
    setPageSize(size)
    setPage(0)
  }, [])

  return {
    displayedData: processedData.displayed,
    totalRows: processedData.totalRows,
    filteredRows: processedData.filteredRows,

    sortColumn,
    sortDirection,
    handleSort,

    searchQuery,
    setSearchQuery: handleSetSearchQuery,

    page,
    pageSize,
    setPage,
    setPageSize: handleSetPageSize,
    totalPages,

    selectedRowKeys,
    toggleRowSelection,
    toggleAllSelection,
    clearSelection,
  }
}

export { useTableState }
export type { UseTableStateProps, UseTableStateReturn }
