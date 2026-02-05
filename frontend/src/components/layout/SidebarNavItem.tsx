import { type ReactNode } from 'react';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import Badge from '@mui/material/Badge';
import Tooltip from '@mui/material/Tooltip';
import { Link as RouterLink } from 'react-router-dom';
import { ANIMATION } from '@/constants';

type SidebarNavItemProps = {
  label: string;
  path: string;
  icon: ReactNode;
  isActive: boolean;
  collapsed: boolean;
  badge?: number;
};

function SidebarNavItem({ label, path, icon, isActive, collapsed, badge }: SidebarNavItemProps) {
  const content = (
    <ListItemButton
      component={RouterLink}
      to={path}
      selected={isActive}
      sx={{
        minHeight: 40,
        px: collapsed ? 2 : 2.5,
        py: 0.75,
        borderRadius: 1.5,
        mx: 1,
        mb: 0.25,
        transition: `all ${ANIMATION.FAST}ms ease`,
        borderLeft: isActive ? '3px solid' : '3px solid transparent',
        borderLeftColor: isActive ? 'primary.main' : 'transparent',
        '&.Mui-selected': {
          backgroundColor: 'action.selected',
          '&:hover': {
            backgroundColor: 'action.hover',
          },
        },
        justifyContent: collapsed ? 'center' : 'flex-start',
      }}
    >
      <ListItemIcon
        sx={{
          minWidth: collapsed ? 0 : 36,
          color: isActive ? 'primary.main' : 'text.secondary',
          justifyContent: 'center',
        }}
      >
        {badge && badge > 0 ? (
          <Badge
            badgeContent={collapsed ? undefined : badge}
            variant={collapsed ? 'dot' : 'standard'}
            color="warning"
            sx={{ '& .MuiBadge-badge': { fontSize: '0.65rem', minWidth: 16, height: 16 } }}
          >
            {icon}
          </Badge>
        ) : (
          icon
        )}
      </ListItemIcon>
      {!collapsed && (
        <ListItemText
          primary={label}
          primaryTypographyProps={{
            fontSize: '0.875rem',
            fontWeight: isActive ? 600 : 400,
            color: isActive ? 'text.primary' : 'text.secondary',
          }}
        />
      )}
    </ListItemButton>
  );

  if (collapsed) {
    return (
      <Tooltip title={label} placement="right" arrow>
        {content}
      </Tooltip>
    );
  }

  return content;
}

export { SidebarNavItem };
