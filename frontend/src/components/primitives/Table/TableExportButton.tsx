import {useState} from 'react'
import {IconButton, Menu, MenuItem, ListItemIcon, ListItemText} from '@mui/material'
import DownloadIcon from '@mui/icons-material/Download'
import TableChartIcon from '@mui/icons-material/TableChart'
import CodeIcon from '@mui/icons-material/Code'

type TableExportButtonProps = {
  onExportCSV: () => void
  onExportJSON: () => void
  disabled?: boolean
}

function TableExportButton({
  onExportCSV,
  onExportJSON,
  disabled = false,
}: TableExportButtonProps) {
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null)
  const open = Boolean(anchorEl)

  const handleClick = (event: React.MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget)
  }

  const handleClose = () => {
    setAnchorEl(null)
  }

  const handleExportCSV = () => {
    onExportCSV()
    handleClose()
  }

  const handleExportJSON = () => {
    onExportJSON()
    handleClose()
  }

  return (
    <>
      <IconButton
        onClick={handleClick}
        disabled={disabled}
        size="small"
        sx={{
          border: 1,
          borderColor: 'divider',
          borderRadius: 1.5,
        }}
        aria-label="Export data"
      >
        <DownloadIcon fontSize="small" />
      </IconButton>

      <Menu anchorEl={anchorEl} open={open} onClose={handleClose}>
        <MenuItem onClick={handleExportCSV}>
          <ListItemIcon>
            <TableChartIcon fontSize="small" />
          </ListItemIcon>
          <ListItemText>Export as CSV</ListItemText>
        </MenuItem>
        <MenuItem onClick={handleExportJSON}>
          <ListItemIcon>
            <CodeIcon fontSize="small" />
          </ListItemIcon>
          <ListItemText>Export as JSON</ListItemText>
        </MenuItem>
      </Menu>
    </>
  )
}

export {TableExportButton}
export type {TableExportButtonProps}
