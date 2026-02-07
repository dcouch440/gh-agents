import { type ReactNode } from 'react';
import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import { Tooltip } from '@/components/primitives/Tooltip';
import Badge from '@mui/material/Badge';
import { LAYOUT, LAYOUT_COLORS, ANIMATION, DESIGN } from '@/constants';
import type { NavBarItem } from './types';

type RailItem = NavBarItem;

type IconRailProps = {
  side: 'left' | 'right';
  topItems: RailItem[];
  bottomItems?: RailItem[];
  footer?: ReactNode;
};

function IconRail({ side, topItems, bottomItems, footer }: IconRailProps) {
  const isLeft = side === 'left';
  const tooltipPlacement = isLeft ? 'right' : 'left';

  const renderItem = (item: RailItem) => {
    return (
      <Tooltip key={item.key} title={item.label} placement={tooltipPlacement}>
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
            filter: item.isActive ? DESIGN.ACTIVE_GLOW : 'none',
            '&:hover': {
              color: item.isActive ? 'primary.light' : 'text.primary',
              backgroundColor: 'transparent',
            },
            '&::before': item.isActive
              ? {
                  content: '""',
                  position: 'absolute',
                  ...(isLeft ? { right: -4 } : { left: -4 }),
                  top: 4,
                  bottom: 4,
                  width: 2,
                  borderRadius: 1,
                  background: DESIGN.ACTIVE_GRADIENT_VERTICAL,
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
      sx={{
        width: LAYOUT.RAIL_WIDTH,
        minWidth: LAYOUT.RAIL_WIDTH,
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        pt: 1,
        pb: 1,
        borderRight: isLeft ? 1 : 0,
        borderLeft: isLeft ? 0 : 1,
        borderColor: 'divider',
        backgroundColor: LAYOUT_COLORS.CHROME_BG,
      }}
    >
      {/* Top items */}
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.25 }}>
        {topItems.map(renderItem)}
      </Box>

      {/* Bottom items — pushed to bottom */}
      <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 0.25, mt: 'auto' }}>
        {bottomItems?.map(renderItem)}
        {footer}
      </Box>
    </Box>
  );
}

export { IconRail };
export type { RailItem, IconRailProps };
