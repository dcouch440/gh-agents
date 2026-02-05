import {Box, Typography} from '@mui/material'
import {SearchInput} from '@/components/primitives'

type TableToolbarProps = {
  searchQuery: string
  onSearchChange: (query: string) => void
  searchPlaceholder?: string
  totalRows: number
  filteredRows: number
}

function TableToolbar({
  searchQuery,
  onSearchChange,
  searchPlaceholder = 'Search...',
  totalRows,
  filteredRows,
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
      <Box sx={{flex: 1, maxWidth: 400}}>
        <SearchInput
          value={searchQuery}
          onChange={onSearchChange}
          placeholder={searchPlaceholder}
        />
      </Box>

      {showResultCount && (
        <Typography variant="body2" color="text.secondary">
          {filteredRows} of {totalRows} results
        </Typography>
      )}
    </Box>
  )
}

export {TableToolbar}
export type {TableToolbarProps}
