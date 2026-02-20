import IconButton from '@mui/material/IconButton'
import { Tooltip } from '@/components/primitives/Tooltip'
import LightModeOutlined from '@mui/icons-material/LightModeOutlined'
import DarkModeOutlined from '@mui/icons-material/DarkModeOutlined'
import ContrastOutlined from '@mui/icons-material/ContrastOutlined'
import { useThemeMode } from '@/hooks/useThemeMode'
import { ANIMATION } from '@/constants'
import { THEMES } from '@/theme'
import type { ThemeId } from '@/theme'

type ThemeToggleProps = {
  tooltipPlacement?: 'top' | 'bottom' | 'left' | 'right'
}

const THEME_ICONS: Record<ThemeId, typeof LightModeOutlined> = {
  linen: LightModeOutlined,
  midnight: DarkModeOutlined,
  slate: ContrastOutlined,
}

function ThemeToggle({ tooltipPlacement = 'bottom' }: ThemeToggleProps) {
  const { themeId, cycleTheme } = useThemeMode()
  const Icon = THEME_ICONS[themeId]
  const label = THEMES[themeId].label

  return (
    <Tooltip title={label} placement={tooltipPlacement}>
      <IconButton
        onClick={cycleTheme}
        size="small"
        sx={{
          color: 'text.secondary',
          transition: `transform ${ANIMATION.NORMAL}ms ease`,
          '&:hover': {
            color: 'text.primary',
            transform: 'rotate(30deg)',
          },
        }}
      >
        <Icon fontSize="small" />
      </IconButton>
    </Tooltip>
  )
}

export { ThemeToggle }
