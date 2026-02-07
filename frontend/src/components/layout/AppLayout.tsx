import { createElement } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { Outlet, Navigate, useLocation, useNavigate } from 'react-router-dom';
import TuneOutlined from '@mui/icons-material/TuneOutlined';
import { TopNavBar } from './TopNavBar';
import { IconRail } from './IconRail';
import { DetailPanel } from './DetailPanel';
import { ThemeToggle } from './ThemeToggle';
import { useStore, authStore, selectUser, selectAuthStatus, layoutStore, reviewQueueStore } from '@/stores';
import { useNavigation } from '@/hooks/useNavigation';
import { LoadingSpinner } from '@/components/primitives';
import { ROUTES } from '@/constants';
import type { NavBarItem } from './types';
import type { RailItem } from './IconRail';

function AppLayout() {
  const status = useStore(authStore.store, selectAuthStatus);
  const user = useStore(authStore.store, selectUser);
  const location = useLocation();
  const navigate = useNavigate();
  const { navItems, utilityItems } = useNavigation();
  const pendingCount = useStore(reviewQueueStore.store, reviewQueueStore.selectPendingCount);
  const rightOpen = useStore(layoutStore.store, layoutStore.selectRightPanelOpen);
  const rightSection = useStore(layoutStore.store, layoutStore.selectRightPanelSection);
  const rightWidth = useStore(layoutStore.store, layoutStore.selectRightPanelWidth);
  const rightDragging = useStore(layoutStore.store, layoutStore.selectRightPanelDragging);

  if (status === 'idle' || status === 'loading') {
    return (
      <Box
        sx={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          minHeight: '100vh',
        }}
      >
        <LoadingSpinner label="Loading..." />
      </Box>
    );
  }

  if (!user) {
    return <Navigate to={ROUTES.LOGIN} state={{ from: location }} replace />;
  }

  const toNavItem = (item: { path: string; icon: React.ReactNode; label: string; isActive: boolean }): NavBarItem => ({
    key: item.path,
    icon: item.icon,
    label: item.label,
    isActive: item.isActive,
    badge: item.path === ROUTES.REVIEW_QUEUE ? pendingCount : undefined,
    onClick: () => {
      void navigate(item.path);
    },
  });

  const topNavItems = navItems.map(toNavItem);
  const utilNavItems = utilityItems.map(toNavItem);

  const showRightRail = location.pathname.startsWith(ROUTES.WORKFLOWS);

  const rightRailItems: RailItem[] = [
    {
      key: 'properties',
      icon: createElement(TuneOutlined, { fontSize: 'small' }),
      label: 'Properties',
      isActive: rightOpen && rightSection === 'properties',
      onClick: () => { layoutStore.toggleRightPanel('properties'); },
    },
  ];

  const rightPanelTitle = rightRailItems.find((i) => i.key === rightSection)?.label ?? '';

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100vh' }}>
      {/* Top: horizontal navigation bar */}
      <TopNavBar navItems={topNavItems} utilityItems={utilNavItems} trailing={<ThemeToggle />} />

      {/* Body: main content + optional right sidebar */}
      <Box sx={{ display: 'flex', flexGrow: 1, minHeight: 0 }}>
        {/* Main content */}
        <Box
          component="main"
          sx={{
            flexGrow: 1,
            overflow: 'auto',
            px: 2.5,
            py: 2,
            minWidth: 0,
          }}
        >
          <Outlet />
        </Box>

        {/* Right detail panel + rail — workflow pages only */}
        {showRightRail && (
          <>
            <DetailPanel
              side="right"
              isOpen={rightOpen}
              onClose={layoutStore.closeRightPanel}
              title={rightPanelTitle}
              width={rightWidth}
              isDragging={rightDragging}
              onResize={layoutStore.setRightPanelWidth}
              onDragStart={layoutStore.startRightPanelDrag}
              onDragEnd={layoutStore.stopRightPanelDrag}
            >
              <Typography variant="body2" color="text.secondary">
                Panel content coming soon.
              </Typography>
            </DetailPanel>
            <IconRail side="right" topItems={rightRailItems} />
          </>
        )}
      </Box>
    </Box>
  );
}

export { AppLayout };
