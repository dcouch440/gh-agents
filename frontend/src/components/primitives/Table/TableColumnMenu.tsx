import { useState } from 'react'
import { IconButton, Menu, MenuItem, Checkbox, ListItemText, Divider, Button, Box } from '@mui/material'
import ViewColumnIcon from '@mui/icons-material/ViewColumn'
import type { TableColumn } from './types'

type TableColumnMenuProps<T> = {
  columns: TableColumn<T>[]
  hiddenColumnKeys: Set<string>
  onToggleColumn: (columnKey: string) => void
  onShowAll: () => void
  onHideAll: () => void
}

function TableColumnMenu<T>({ columns, hiddenColumnKeys, onToggleColumn, onShowAll, onHideAll }: TableColumnMenuProps<T>) {
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null)
  const open = Boolean(anchorEl)

  const handleClick = (event: React.MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget)
  }

  const handleClose = () => {
    setAnchorEl(null)
  }

  const visibleCount = columns.length - hiddenColumnKeys.size

  return (
    <>
      <IconButton
        onClick={handleClick}
        size="small"
        sx={{
          border: 1,
          borderColor: 'divider',
          borderRadius: 1.5,
        }}
        aria-label="Column visibility"
      >
        <ViewColumnIcon fontSize="small" />
      </IconButton>

      <Menu
        anchorEl={anchorEl}
        open={open}
        onClose={handleClose}
        PaperProps={{
          sx: { minWidth: 200 },
        }}
      >
        <Box sx={{ px: 2, py: 1 }}>
          <Box sx={{ display: 'flex', gap: 1, mb: 1 }}>
            <Button size="small" onClick={onShowAll} fullWidth variant="outlined">
              Show All
            </Button>
            <Button size="small" onClick={onHideAll} fullWidth variant="outlined">
              Hide All
            </Button>
          </Box>
        </Box>

        <Divider />

        {columns.map((column) => {
          const isVisible = !hiddenColumnKeys.has(column.key)
          const isLastVisible = visibleCount === 1 && isVisible

          return (
            <MenuItem key={column.key} onClick={() => !isLastVisible && onToggleColumn(column.key)} disabled={isLastVisible}>
              <Checkbox checked={isVisible} disabled={isLastVisible} sx={{ p: 0, mr: 1 }} />
              <ListItemText primary={column.header} />
            </MenuItem>
          )
        })}
      </Menu>
    </>
  )
}

export { TableColumnMenu }
export type { TableColumnMenuProps }
