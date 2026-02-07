import IconButton from '@mui/material/IconButton';
import { Tooltip } from '@/components/primitives/Tooltip';
import LightModeOutlined from '@mui/icons-material/LightModeOutlined';
import DarkModeOutlined from '@mui/icons-material/DarkModeOutlined';
import { useThemeMode } from '@/hooks/useThemeMode';
import { ANIMATION } from '@/constants';

function ThemeToggle() {
  const { mode, toggleMode } = useThemeMode();
  const isDark = mode === 'dark';

  return (
    <Tooltip title={isDark ? 'Switch to light mode' : 'Switch to dark mode'} placement="right">
      <IconButton
        onClick={toggleMode}
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
        {isDark ? <LightModeOutlined fontSize="small" /> : <DarkModeOutlined fontSize="small" />}
      </IconButton>
    </Tooltip>
  );
}

export { ThemeToggle };
