import { useState } from 'react'
import Box from '@mui/material/Box'
import IconButton from '@mui/material/IconButton'
import ListItemIcon from '@mui/material/ListItemIcon'
import ListItemText from '@mui/material/ListItemText'
import Menu from '@mui/material/Menu'
import MenuItem from '@mui/material/MenuItem'
import PaletteOutlined from '@mui/icons-material/PaletteOutlined'
import Check from '@mui/icons-material/Check'
import { Tooltip } from '@/components/primitives/Tooltip'
import { useThemeMode } from '@/hooks/useThemeMode'
import { ANIMATION } from '@/constants'
import { THEME_LIST } from '@/theme'

type ThemeToggleProps = {
  tooltipPlacement?: 'top' | 'bottom' | 'left' | 'right'
}

function ThemeToggle({ tooltipPlacement = 'bottom' }: ThemeToggleProps) {
  const { themeId, setTheme } = useThemeMode()
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
  const open = anchorEl !== null

  const handleOpen = (event: React.MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget)
  }

  const handleClose = () => {
    setAnchorEl(null)
  }

  return (
    <>
      <Tooltip title="Theme" placement={tooltipPlacement}>
        <IconButton
          onClick={handleOpen}
          size="small"
          aria-label="Theme"
          aria-controls={open ? 'theme-menu' : undefined}
          aria-haspopup="true"
          aria-expanded={open ? 'true' : undefined}
          sx={{
            color: 'text.secondary',
            transition: `color ${ANIMATION.NORMAL}ms ease`,
            '&:hover': { color: 'text.primary' },
          }}
        >
          <PaletteOutlined fontSize="small" />
        </IconButton>
      </Tooltip>

      <Menu
        id="theme-menu"
        anchorEl={anchorEl}
        open={open}
        onClose={handleClose}
        slotProps={{ paper: { sx: { minWidth: 160 } } }}
      >
        {THEME_LIST.map((def) => (
          <MenuItem
            key={def.id}
            selected={def.id === themeId}
            onClick={() => {
              setTheme(def.id)
              handleClose()
            }}
          >
            <ListItemIcon>
              <Box
                sx={{
                  width: 16,
                  height: 16,
                  borderRadius: '50%',
                  backgroundColor: def.custom.canvasBg,
                  border: `1.5px solid ${def.custom.strokeColor}`,
                  flexShrink: 0,
                }}
              />
            </ListItemIcon>
            <ListItemText primary={def.label} />
            {def.id === themeId && (
              <Check fontSize="small" sx={{ ml: 1, color: 'text.secondary' }} />
            )}
          </MenuItem>
        ))}
      </Menu>
    </>
  )
}

export { ThemeToggle }
