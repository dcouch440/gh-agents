import { type ReactNode } from 'react';
import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import { Tooltip } from '@/components/primitives/Tooltip';
import Badge from '@mui/material/Badge';
import { useTheme } from '@mui/material/styles';
import { LAYOUT, ANIMATION } from '@/constants';
import type { NavBarItem } from './types';

type TopNavBarProps = {
  navItems: NavBarItem[];
  utilityItems: NavBarItem[];
  trailing?: ReactNode;
};

function TopNavBar({ navItems, utilityItems, trailing }: TopNavBarProps) {
  const theme = useTheme();

  const renderItem = (item: NavBarItem) => {
    return (
      <Tooltip key={item.key} title={item.label} placement="bottom">
        <IconButton
          onClick={item.onClick}
          sx={{
            width: 28,
            height: 28,
            borderRadius: '6px',
            position: 'relative',
            color: item.isActive ? 'primary.main' : 'text.secondary',
            backgroundColor: 'transparent',
            transition: `color ${ANIMATION.FAST}ms ease, filter ${ANIMATION.FAST}ms ease`,
            filter: item.isActive ? theme.palette.custom.activeGlow : 'none',
            '&:hover': {
              color: item.isActive ? 'primary.light' : 'text.primary',
              backgroundColor: 'transparent',
            },
            '&::after': item.isActive
              ? {
                  content: '""',
                  position: 'absolute',
                  bottom: -4,
                  left: 4,
                  right: 4,
                  height: 2,
                  borderRadius: 1,
                  background: theme.palette.custom.activeGradient,
                }
              : undefined,
          }}
        >
          {item.badge && item.badge > 0 ? (
            <Badge
              variant="dot"
              color="warning"
              sx={{ '& .MuiBadge-badge': { top: 2, right: 2, width: 6, height: 6, minWidth: 6 } }}
            >
              {item.icon}
            </Badge>
          ) : (
            item.icon
          )}
        </IconButton>
      </Tooltip>
    );
  };

  return (
    <Box
      component="nav"
      sx={{
        height: LAYOUT.TOPBAR_HEIGHT,
        minHeight: LAYOUT.TOPBAR_HEIGHT,
        display: 'flex',
        alignItems: 'center',
        px: 1,
        borderBottom: 1,
        borderColor: 'divider',
        backgroundColor: theme.palette.custom.chromeBg,
        boxShadow: '0 1px 2px rgba(0, 0, 0, 0.2)',
        zIndex: 10,
        position: 'relative',
        gap: 0.25,
      }}
    >
      {/* Left: navigation items */}
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.25 }}>
        {navItems.map(renderItem)}
      </Box>

      {/* Spacer */}
      <Box sx={{ flexGrow: 1 }} />

      {/* Right: utility items + trailing (ThemeToggle) */}
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.25 }}>
        {utilityItems.map(renderItem)}
        {trailing}
      </Box>
    </Box>
  );
}

export { TopNavBar };
export type { TopNavBarProps };
