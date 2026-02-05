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
} from '@mui/material'
import {LoadingSpinner, EmptyState, ErrorMessage, Skeleton} from '@/components/primitives'
import {useTableState} from './useTableState'
import {TableToolbar} from './TableToolbar'
import {TablePagination} from './TablePagination'
import {getDensityPadding} from './utils'
import type {TableProps, TableColumn} from './types'

function Table<T>({
  data,
  keyExtractor,
  columns,
  loading = false,
  error = null,
  emptyMessage = 'No data available',
  enableSorting = false,
  enableSearch = false,
  enablePagination = false,
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
  } = useTableState({
    data,
    columns,
    defaultSortColumn,
    defaultSortDirection,
    defaultPageSize,
    searchFields,
    enableSorting,
    enableSearch,
    enablePagination,
  })

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

  return (
    <TableContainer component={Paper} elevation={0}>
      {enableSearch && (
        <TableToolbar
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
          searchPlaceholder={searchPlaceholder}
          totalRows={totalRows}
          filteredRows={filteredRows}
        />
      )}
      <MuiTable
        size={density === 'compact' ? 'small' : 'medium'}
        stickyHeader={stickyHeader}
      >
        <TableHead>
          <TableRow>
            {columns.map((column) => {
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
                {columns.map((column) => (
                  <TableCell key={column.key} sx={{p: padding}}>
                    <Skeleton variant="text" />
                  </TableCell>
                ))}
              </TableRow>
            ))
          ) : displayedData.length === 0 ? (
            // No filtered results
            <TableRow>
              <TableCell colSpan={columns.length} align="center" sx={{py: 4}}>
                <EmptyState message="No results found" />
              </TableCell>
            </TableRow>
          ) : (
            // Data rows
            displayedData.map((row) => {
              const rowKey = keyExtractor(row)
              return (
                <TableRow
                  key={rowKey}
                  hover
                  onClick={onRowClick ? () => onRowClick(row) : undefined}
                  sx={{
                    cursor: onRowClick ? 'pointer' : 'default',
                  }}
                >
                  {columns.map((column) => (
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
