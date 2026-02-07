import { type ReactNode } from 'react';
import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';
import Badge from '@mui/material/Badge';
import Divider from '@mui/material/Divider';
import { LAYOUT, ANIMATION } from '@/constants';

type RailItem = {
  key: string;
  icon: ReactNode;
  label: string;
  isActive: boolean;
  badge?: number;
  onClick: () => void;
};

type IconRailProps = {
  side: 'left' | 'right';
  topItems: RailItem[];
  bottomItems?: RailItem[];
};

function IconRail({ side, topItems, bottomItems }: IconRailProps) {
  const isLeft = side === 'left';
  const tooltipPlacement = isLeft ? 'right' : 'left';

  const renderItem = (item: RailItem) => {
    return (
      <Tooltip key={item.key} title={item.label} placement={tooltipPlacement}>
        <Box
          sx={{
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            py: 0.25,
          }}
        >
          <IconButton
            onClick={item.onClick}
            sx={{
              width: 30,
              height: 30,
              borderRadius: '6px',
              color: item.isActive ? 'primary.main' : 'text.secondary',
              backgroundColor: item.isActive ? 'rgba(59, 130, 246, 0.08)' : 'transparent',
              transition: `all ${ANIMATION.FAST}ms ease`,
              '&:hover': {
                color: item.isActive ? 'primary.main' : 'text.primary',
                backgroundColor: item.isActive ? 'rgba(59, 130, 246, 0.12)' : 'action.hover',
              },
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
        </Box>
      </Tooltip>
    );
  };

  return (
    <Box
      sx={{
        width: LAYOUT.RAIL_WIDTH,
        minWidth: LAYOUT.RAIL_WIDTH,
        height: '100vh',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        pt: 1.5,
        pb: 1,
        borderRight: isLeft ? 1 : 0,
        borderLeft: isLeft ? 0 : 1,
        borderColor: 'divider',
        backgroundColor: 'background.default',
        overflow: 'hidden',
        position: 'sticky',
        top: 0,
      }}
    >
      {/* Top items */}
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.25, flexGrow: 1 }}>
        {topItems.map(renderItem)}
      </Box>

      {/* Bottom items */}
      {bottomItems && bottomItems.length > 0 && (
        <>
          <Divider sx={{ width: '50%', my: 0.5 }} />
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.25 }}>
            {bottomItems.map(renderItem)}
          </Box>
        </>
      )}
    </Box>
  );
}

export { IconRail };
export type { RailItem, IconRailProps };
