import type { ReactNode } from 'react'

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

function DataTable<T>({
  columns,
  rows,
  rowKey,
  sortColumn,
  sortDirection,
  onSort,
}: DataTableProps<T>) {
  return (
    <table className="table">
      <thead>
        <tr>
          {columns.map((col) => {
            const isSortable = col.sortable === true && onSort !== undefined && onSort !== null
            const isActive = sortColumn === col.key
            return (
              <th
                key={col.key}
                className={isSortable ? 'th--sortable' : undefined}
                onClick={isSortable ? () => onSort(col.key) : undefined}
              >
                {col.header}
                {isActive && sortDirection ? (sortDirection === 'asc' ? ' ↑' : ' ↓') : null}
              </th>
            )
          })}
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={rowKey(row)}>
            {columns.map((col) => (
              <td key={col.key}>{col.render(row)}</td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  )
}

export { DataTable }
export type { Column, SortDirection, DataTableProps }
