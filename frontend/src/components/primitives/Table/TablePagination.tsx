import {TablePagination as MuiTablePagination} from '@mui/material'

type TablePaginationProps = {
  count: number
  page: number
  rowsPerPage: number
  onPageChange: (page: number) => void
  onRowsPerPageChange: (size: number) => void
  rowsPerPageOptions?: number[]
}

function TablePagination({
  count,
  page,
  rowsPerPage,
  onPageChange,
  onRowsPerPageChange,
  rowsPerPageOptions = [10, 25, 50, 100],
}: TablePaginationProps) {
  return (
    <MuiTablePagination
      component="div"
      count={count}
      page={page}
      onPageChange={(_, newPage) => onPageChange(newPage)}
      rowsPerPage={rowsPerPage}
      onRowsPerPageChange={(e) => onRowsPerPageChange(parseInt(e.target.value, 10))}
      rowsPerPageOptions={rowsPerPageOptions}
      sx={{
        borderTop: 1,
        borderColor: 'divider',
      }}
    />
  )
}

export {TablePagination}
export type {TablePaginationProps}
