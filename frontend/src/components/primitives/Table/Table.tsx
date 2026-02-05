import {
  Table as MuiTable,
  TableHead,
  TableBody,
  TableRow,
  TableCell,
  TableContainer,
  TableSortLabel,
  Paper,
  Box,
  Checkbox,
} from '@mui/material'
import {useCallback} from 'react'
import {LoadingSpinner, EmptyState, ErrorMessage, Skeleton} from '@/components/primitives'
import {useTableState} from './useTableState'
import {useTableColumns} from './useTableColumns'
import {TableToolbar} from './TableToolbar'
import {TablePagination} from './TablePagination'
import {TableColumnMenu} from './TableColumnMenu'
import {TableExportButton} from './TableExportButton'
import {getDensityPadding, exportToCSV, exportToJSON} from './utils'
import type {TableProps, TableColumn} from './types'

function Table<T>({
  data,
  keyExtractor,
  columns,
  defaultVisibleColumns,
  loading = false,
  error = null,
  emptyMessage = 'No data available',
  enableSorting = false,
  enableSearch = false,
  enablePagination = false,
  enableSelection = false,
  enableExport = false,
  exportFilename,
  selectionMode = 'multiple',
  selectedRows = [],
  onSelectionChange,
  defaultSortColumn,
  defaultSortDirection = 'asc',
  defaultPageSize = 25,
  pageSizeOptions,
  searchPlaceholder,
  searchFields,
  stickyHeader = false,
  density = 'normal',
  onRowClick,
}: TableProps<T>) {
  // Column visibility management
  const {
    visibleColumns,
    hiddenColumnKeys,
    toggleColumnVisibility,
    showAllColumns,
    hideAllColumns,
  } = useTableColumns({
    columns,
    defaultVisibleColumns,
  })

  const {
    displayedData,
    totalRows,
    filteredRows,
    sortColumn,
    sortDirection,
    handleSort,
    searchQuery,
    setSearchQuery,
    page,
    pageSize,
    setPage,
    setPageSize,
    selectedRowKeys,
    toggleRowSelection,
    toggleAllSelection,
  } = useTableState({
    data,
    columns: visibleColumns,
    defaultSortColumn,
    defaultSortDirection,
    defaultPageSize,
    searchFields,
    enableSorting,
    enableSearch,
    enablePagination,
  })

  // Export handlers (must be defined before early returns)
  const handleExportCSV = useCallback(() => {
    const dataToExport = displayedData
    const columnsForExport = visibleColumns.map((col) => ({
      key: col.key,
      header: col.header,
    }))
    const filename = exportFilename ? `${exportFilename}.csv` : 'export.csv'
    exportToCSV(dataToExport, columnsForExport, filename)
  }, [displayedData, visibleColumns, exportFilename])

  const handleExportJSON = useCallback(() => {
    const filename = exportFilename ? `${exportFilename}.json` : 'export.json'
    exportToJSON(displayedData, filename)
  }, [displayedData, exportFilename])

  // Loading state
  if (loading && data.length === 0) {
    return (
      <Box sx={{p: 4}}>
        <LoadingSpinner centered label="Loading data..." />
      </Box>
    )
  }

  // Error state
  if (error) {
    return (
      <Box sx={{p: 2}}>
        <ErrorMessage message={error} />
      </Box>
    )
  }

  // Empty state
  if (!loading && totalRows === 0) {
    return (
      <Box sx={{p: 4}}>
        <EmptyState message={emptyMessage} />
      </Box>
    )
  }

  const padding = getDensityPadding(density)

  // Sync selection with parent component
  if (enableSelection && onSelectionChange) {
    const currentSelection = Array.from(selectedRowKeys)
    const isDifferent =
      currentSelection.length !== selectedRows.length ||
      currentSelection.some((key) => !selectedRows.includes(key))

    if (isDifferent) {
      onSelectionChange(currentSelection)
    }
  }

  // Check if all displayed rows are selected
  const allDisplayedKeys = displayedData.map((row) => keyExtractor(row))
  const allSelected =
    enableSelection &&
    allDisplayedKeys.length > 0 &&
    allDisplayedKeys.every((key) => selectedRowKeys.has(key))

  const someSelected =
    enableSelection &&
    allDisplayedKeys.some((key) => selectedRowKeys.has(key)) &&
    !allSelected

  return (
    <TableContainer component={Paper} elevation={0}>
      {enableSearch && (
        <TableToolbar
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
          searchPlaceholder={searchPlaceholder}
          totalRows={totalRows}
          filteredRows={filteredRows}
          columnMenu={
            <TableColumnMenu
              columns={columns}
              hiddenColumnKeys={hiddenColumnKeys}
              onToggleColumn={toggleColumnVisibility}
              onShowAll={showAllColumns}
              onHideAll={hideAllColumns}
            />
          }
          exportButton={
            enableExport ? (
              <TableExportButton
                onExportCSV={handleExportCSV}
                onExportJSON={handleExportJSON}
                disabled={displayedData.length === 0}
              />
            ) : undefined
          }
        />
      )}
      <MuiTable
        size={density === 'compact' ? 'small' : 'medium'}
        stickyHeader={stickyHeader}
      >
        <TableHead>
          <TableRow>
            {enableSelection && (
              <TableCell padding="checkbox" sx={{p: padding}}>
                {selectionMode === 'multiple' && (
                  <Checkbox
                    indeterminate={someSelected}
                    checked={allSelected}
                    onChange={() => toggleAllSelection(allDisplayedKeys)}
                    inputProps={{'aria-label': 'Select all rows'}}
                  />
                )}
              </TableCell>
            )}
            {visibleColumns.map((column) => {
              const isSortable = enableSorting && column.sortable
              const isActive = sortColumn === column.key

              return (
                <TableCell
                  key={column.key}
                  align={column.align ?? 'left'}
                  sx={{
                    width: column.width,
                    minWidth: column.minWidth,
                    maxWidth: column.maxWidth,
                    p: padding,
                    fontWeight: 600,
                  }}
                >
                  {isSortable ? (
                    <TableSortLabel
                      active={isActive}
                      direction={isActive ? sortDirection : 'asc'}
                      onClick={() => handleSort(column.key)}
                    >
                      {column.header}
                    </TableSortLabel>
                  ) : (
                    column.header
                  )}
                </TableCell>
              )
            })}
          </TableRow>
        </TableHead>
        <TableBody>
          {loading ? (
            // Loading skeleton rows
            Array.from({length: 5}).map((_, index) => (
              <TableRow key={index}>
                {enableSelection && (
                  <TableCell padding="checkbox" sx={{p: padding}}>
                    <Skeleton variant="rectangular" width={24} height={24} />
                  </TableCell>
                )}
                {visibleColumns.map((column) => (
                  <TableCell key={column.key} sx={{p: padding}}>
                    <Skeleton variant="text" />
                  </TableCell>
                ))}
              </TableRow>
            ))
          ) : displayedData.length === 0 ? (
            // No filtered results
            <TableRow>
              <TableCell
                colSpan={visibleColumns.length + (enableSelection ? 1 : 0)}
                align="center"
                sx={{py: 4}}
              >
                <EmptyState message="No results found" />
              </TableCell>
            </TableRow>
          ) : (
            // Data rows
            displayedData.map((row) => {
              const rowKey = keyExtractor(row)
              const isSelected = selectedRowKeys.has(rowKey)
              return (
                <TableRow
                  key={rowKey}
                  hover
                  selected={enableSelection && isSelected}
                  onClick={onRowClick ? () => onRowClick(row) : undefined}
                  sx={{
                    cursor: onRowClick ? 'pointer' : 'default',
                  }}
                >
                  {enableSelection && (
                    <TableCell padding="checkbox" sx={{p: padding}}>
                      <Checkbox
                        checked={isSelected}
                        onChange={() => toggleRowSelection(rowKey)}
                        onClick={(e) => e.stopPropagation()}
                        inputProps={{'aria-label': `Select row ${rowKey}`}}
                      />
                    </TableCell>
                  )}
                  {visibleColumns.map((column) => (
                    <TableCell
                      key={column.key}
                      align={column.align ?? 'left'}
                      sx={{
                        p: padding,
                        ...(column.truncate && {
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                          maxWidth: column.maxWidth ?? column.width ?? 300,
                        }),
                      }}
                    >
                      {column.render ? (
                        column.render(row)
                      ) : (
                        (() => {
                          const value = (row as Record<string, unknown>)[column.key]
                          if (value === null || value === undefined) return ''
                          if (typeof value === 'object') return JSON.stringify(value)
                          // At this point, value is a primitive (string, number, boolean, bigint, symbol)
                          return String(value as string | number | boolean | bigint | symbol)
                        })()
                      )}
                    </TableCell>
                  ))}
                </TableRow>
              )
            })
          )}
        </TableBody>
      </MuiTable>
      {enablePagination && (
        <TablePagination
          count={filteredRows}
          page={page}
          rowsPerPage={pageSize}
          onPageChange={setPage}
          onRowsPerPageChange={setPageSize}
          rowsPerPageOptions={pageSizeOptions}
        />
      )}
    </TableContainer>
  )
}

export {Table}
export type {TableProps, TableColumn}
