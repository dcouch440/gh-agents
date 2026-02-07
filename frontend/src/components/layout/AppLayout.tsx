import Box from '@mui/material/Box';
import { Outlet, Navigate, useLocation, useNavigate } from 'react-router-dom';
import { IconRail } from './IconRail';
import { DetailPanel } from './DetailPanel';
import { ThemeToggle } from './ThemeToggle';
import { useStore, authStore, selectUser, selectAuthStatus, layoutStore, reviewQueueStore } from '@/stores';
import { useNavigation } from '@/hooks/useNavigation';
import { LoadingSpinner } from '@/components/primitives';
import { ROUTES } from '@/constants';
import type { RailItem } from './IconRail';

function AppLayout() {
  const status = useStore(authStore.store, selectAuthStatus);
  const user = useStore(authStore.store, selectUser);
  const location = useLocation();
  const navigate = useNavigate();
  const { navItems, utilityItems } = useNavigation();
  const leftOpen = useStore(layoutStore.store, layoutStore.selectLeftPanelOpen);
  const leftSection = useStore(layoutStore.store, layoutStore.selectLeftPanelSection);
  const pendingCount = useStore(reviewQueueStore.store, reviewQueueStore.selectPendingCount);

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

  const toRailItem = (item: { path: string; icon: React.ReactNode; label: string; isActive: boolean }): RailItem => ({
    key: item.path,
    icon: item.icon,
    label: item.label,
    isActive: item.isActive,
    badge: item.path === ROUTES.REVIEW_QUEUE ? pendingCount : undefined,
    onClick: () => {
      if (item.isActive) {
        // Already on this page — toggle the detail panel
        layoutStore.toggleLeftPanel(item.path);
      } else {
        // Navigate to the page, close any open panel
        layoutStore.closeLeftPanel();
        void navigate(item.path);
      }
    },
  });

  const topRailItems = navItems.map(toRailItem);
  const bottomRailItems = utilityItems.map(toRailItem);

  const panelTitle = [...navItems, ...utilityItems]
    .find((i) => i.path === leftSection)?.label ?? '';

  return (
    <Box sx={{ display: 'flex', minHeight: '100vh' }}>
      {/* Left: icon rail + detail panel */}
      <IconRail side="left" topItems={topRailItems} bottomItems={bottomRailItems} footer={<ThemeToggle />} />
      <DetailPanel
        side="left"
        isOpen={leftOpen}
        onClose={layoutStore.closeLeftPanel}
        title={panelTitle}
      />

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
    </Box>
  );
}

export { AppLayout };
