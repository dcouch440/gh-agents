import { Box, Typography } from '@mui/material'
import { SearchInput } from '@/components/primitives'
import type { ReactNode } from 'react'

type TableToolbarProps = {
  searchQuery: string
  onSearchChange: (query: string) => void
  searchPlaceholder?: string
  totalRows: number
  filteredRows: number
  columnMenu?: ReactNode
  exportButton?: ReactNode
}

function TableToolbar({
  searchQuery,
  onSearchChange,
  searchPlaceholder = 'Search...',
  totalRows,
  filteredRows,
  columnMenu,
  exportButton,
}: TableToolbarProps) {
  const showResultCount = searchQuery.trim().length > 0 && filteredRows !== totalRows

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: 2,
        p: 2,
        borderBottom: 1,
        borderColor: 'divider',
      }}
    >
      <Box sx={{ flex: 1, maxWidth: 400 }}>
        <SearchInput value={searchQuery} onChange={onSearchChange} placeholder={searchPlaceholder} />
      </Box>

      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
        {showResultCount && (
          <Typography variant="body2" color="text.secondary">
            {filteredRows} of {totalRows} results
          </Typography>
        )}
        {exportButton}
        {columnMenu}
      </Box>
    </Box>
  )
}

export { TableToolbar }
export type { TableToolbarProps }
