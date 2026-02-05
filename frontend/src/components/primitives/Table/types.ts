import type {ReactNode} from 'react'

type SortDirection = 'asc' | 'desc'

type TableColumn<T> = {
  key: string
  header: string
  render?: (row: T) => ReactNode
  sortable?: boolean
  sortFn?: (a: T, b: T) => number
  filterable?: boolean
  width?: number | string
  minWidth?: number
  maxWidth?: number
  align?: 'left' | 'center' | 'right'
  truncate?: boolean
}

type RowAction<T> = {
  key: string
  label: string
  icon?: ReactNode
  onClick: (row: T) => void | Promise<void>
  disabled?: (row: T) => boolean
  hidden?: (row: T) => boolean
  variant?: 'text' | 'outlined' | 'contained'
  color?: 'primary' | 'secondary' | 'error' | 'warning' | 'success'
}

type TableDensity = 'compact' | 'normal' | 'comfortable'

type TableProps<T> = {
  // Data
  data: T[]
  keyExtractor: (row: T) => string

  // Columns
  columns: TableColumn<T>[]
  defaultVisibleColumns?: string[]

  // States
  loading?: boolean
  error?: string | null
  emptyMessage?: string

  // Features (opt-in)
  enableSorting?: boolean
  enableFiltering?: boolean
  enableSearch?: boolean
  enablePagination?: boolean
  enableSelection?: boolean
  enableRowActions?: boolean

  // Sorting
  defaultSortColumn?: string
  defaultSortDirection?: SortDirection
  onSortChange?: (column: string, direction: SortDirection) => void

  // Pagination
  defaultPageSize?: number
  pageSizeOptions?: number[]

  // Selection
  selectionMode?: 'single' | 'multiple'
  selectedRows?: string[]
  onSelectionChange?: (selectedKeys: string[]) => void

  // Row Actions
  rowActions?: RowAction<T>[]

  // Search
  searchPlaceholder?: string
  searchFields?: (keyof T)[]

  // Styling
  stickyHeader?: boolean
  density?: TableDensity

  // Callbacks
  onRowClick?: (row: T) => void
}

type TableState = {
  // Sort state
  sortColumn: string | null
  sortDirection: SortDirection

  // Search state
  searchQuery: string

  // Pagination state
  page: number
  pageSize: number

  // Selection state
  selectedRowKeys: Set<string>

  // Column visibility state
  visibleColumns: Set<string>
}

export type {
  SortDirection,
  TableColumn,
  RowAction,
  TableDensity,
  TableProps,
  TableState,
}
