import { useState } from 'react'
import { IconButton, Menu, MenuItem, ListItemIcon, ListItemText, Divider } from '@mui/material'
import MoreVertIcon from '@mui/icons-material/MoreVert'
import type { ReactNode } from 'react'

type MenuAction = {
  key: string
  label: string
  icon?: ReactNode
  onClick: () => void | Promise<void>
  disabled?: boolean
  color?: 'default' | 'error' | 'warning' | 'success'
  dividerAfter?: boolean
}

type ActionMenuProps = {
  actions: MenuAction[]
  ariaLabel?: string
  size?: 'small' | 'medium'
}

function ActionMenu({ actions, ariaLabel = 'Actions', size = 'small' }: ActionMenuProps) {
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null)
  const open = Boolean(anchorEl)

  const handleClick = (event: React.MouseEvent<HTMLElement>) => {
    event.stopPropagation()
    setAnchorEl(event.currentTarget)
  }

  const handleClose = () => {
    setAnchorEl(null)
  }

  const handleActionClick = async (action: MenuAction) => {
    handleClose()
    const result = action.onClick()
    if (result instanceof Promise) {
      await result
    }
  }

  const visibleActions = actions.filter((action) => !action.disabled)

  if (visibleActions.length === 0) {
    return null
  }

  return (
    <>
      <IconButton
        onClick={handleClick}
        size={size}
        aria-label={ariaLabel}
        aria-controls={open ? 'action-menu' : undefined}
        aria-haspopup="true"
        aria-expanded={open ? 'true' : undefined}
      >
        <MoreVertIcon fontSize={size} />
      </IconButton>

      <Menu
        id="action-menu"
        anchorEl={anchorEl}
        open={open}
        onClose={handleClose}
        onClick={(e) => e.stopPropagation()}
        PaperProps={{
          sx: { minWidth: 180 },
        }}
      >
        {actions.map((action, index) => (
          <div key={action.key}>
            <MenuItem
              onClick={() => {
                void handleActionClick(action)
              }}
              disabled={action.disabled}
              sx={{
                color: action.color === 'error' ? 'error.main' : undefined,
              }}
            >
              {action.icon && <ListItemIcon>{action.icon}</ListItemIcon>}
              <ListItemText primary={action.label} />
            </MenuItem>
            {action.dividerAfter && index < actions.length - 1 && <Divider />}
          </div>
        ))}
      </Menu>
    </>
  )
}

export { ActionMenu }
export type { ActionMenuProps, MenuAction }
