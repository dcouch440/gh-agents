import Box from '@mui/material/Box';
import Drawer from '@mui/material/Drawer';
import IconButton from '@mui/material/IconButton';
import List from '@mui/material/List';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import Tooltip from '@mui/material/Tooltip';
import ChevronLeftOutlined from '@mui/icons-material/ChevronLeftOutlined';
import ChevronRightOutlined from '@mui/icons-material/ChevronRightOutlined';
import { useNavigation } from '@/hooks/useNavigation';
import { useSidebar } from '@/hooks/useSidebar';
import { useStore, reviewQueueStore } from '@/stores';
import { SidebarNavItem } from './SidebarNavItem';
import { ThemeToggle } from './ThemeToggle';
import { APP_NAME, ROUTES, SIDEBAR, ANIMATION } from '@/constants';

function Sidebar() {
  const { collapsed, toggle } = useSidebar();
  const { navItems } = useNavigation();
  const pendingCount = useStore(reviewQueueStore.store, reviewQueueStore.selectPendingCount);
  const width = collapsed ? SIDEBAR.WIDTH_COLLAPSED : SIDEBAR.WIDTH_EXPANDED;

  return (
    <Drawer
      variant="permanent"
      sx={{
        width,
        flexShrink: 0,
        '& .MuiDrawer-paper': {
          width,
          boxSizing: 'border-box',
          transition: `width ${ANIMATION.NORMAL}ms ease`,
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
        },
      }}
    >
      {/* Logo header */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: collapsed ? 'center' : 'flex-start',
          px: collapsed ? 1 : 2.5,
          py: 2,
          minHeight: 56,
        }}
      >
        <Typography
          variant="h6"
          sx={{
            fontWeight: 700,
            letterSpacing: '-0.02em',
            color: 'text.primary',
            overflow: 'hidden',
            whiteSpace: 'nowrap',
          }}
        >
          {collapsed ? 'n' : APP_NAME}
        </Typography>
      </Box>

      <Divider />

      {/* Navigation items */}
      <List sx={{ flexGrow: 1, pt: 1 }}>
        {navItems.map((item) => (
          <SidebarNavItem
            key={item.path}
            label={item.label}
            path={item.path}
            icon={item.icon}
            isActive={item.isActive}
            collapsed={collapsed}
            badge={item.path === ROUTES.REVIEW_QUEUE ? pendingCount : undefined}
          />
        ))}
      </List>

      {/* Bottom section */}
      <Box sx={{ p: 1 }}>
        <Divider sx={{ mb: 1 }} />
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: collapsed ? 'center' : 'space-between',
            px: collapsed ? 0 : 1,
          }}
        >
          <ThemeToggle />
          <Tooltip title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'} placement="right">
            <IconButton
              onClick={toggle}
              size="small"
              sx={{
                color: 'text.secondary',
                '&:hover': { color: 'text.primary' },
              }}
            >
              {collapsed ? <ChevronRightOutlined fontSize="small" /> : <ChevronLeftOutlined fontSize="small" />}
            </IconButton>
          </Tooltip>
        </Box>
      </Box>
    </Drawer>
  );
}

export { Sidebar };
