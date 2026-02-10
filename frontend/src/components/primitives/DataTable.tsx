import type { ReactNode } from 'react'
import { Table, TableHead, TableBody, TableRow, TableCell, TableSortLabel, TableContainer, Paper } from '@mui/material'

type SortDirection = 'asc' | 'desc'

type Column<T> = {
  key: string
  header: string
  sortable?: boolean
  render: (row: T) => ReactNode
}

type DataTableProps<T> = {
  columns: Column<T>[]
  rows: T[]
  rowKey: (row: T) => string
  sortColumn?: string | null
  sortDirection?: SortDirection
  onSort?: ((columnKey: string) => void) | null
}

function DataTable<T>({ columns, rows, rowKey, sortColumn, sortDirection, onSort }: DataTableProps<T>) {
  return (
    <TableContainer component={Paper} elevation={0}>
      <Table size="small">
        <TableHead>
          <TableRow>
            {columns.map((col) => {
              const isSortable = col.sortable === true && onSort !== undefined && onSort !== null
              const isActive = sortColumn === col.key
              return (
                <TableCell key={col.key}>
                  {isSortable ? (
                    <TableSortLabel
                      active={isActive}
                      direction={isActive && sortDirection ? sortDirection : 'asc'}
                      onClick={() => onSort(col.key)}
                    >
                      {col.header}
                    </TableSortLabel>
                  ) : (
                    col.header
                  )}
                </TableCell>
              )
            })}
          </TableRow>
        </TableHead>
        <TableBody>
          {rows.map((row) => (
            <TableRow key={rowKey(row)} hover>
              {columns.map((col) => (
                <TableCell key={col.key}>{col.render(row)}</TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </TableContainer>
  )
}

export { DataTable }
export type { Column, SortDirection, DataTableProps }
