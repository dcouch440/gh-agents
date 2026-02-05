import Box from '@mui/material/Box';
import Container from '@mui/material/Container';
import { Outlet, Navigate, useLocation } from 'react-router-dom';
import { Sidebar } from './Sidebar';
import { useSidebar } from '@/hooks/useSidebar';
import { useAuth } from '@/hooks/useAuth';
import { LoadingSpinner } from '@/components/primitives';
import { SIDEBAR, ANIMATION, ROUTES } from '@/constants';

function AppLayout() {
  const { user, loading } = useAuth();
  const { collapsed } = useSidebar();
  const location = useLocation();
  const sidebarWidth = collapsed ? SIDEBAR.WIDTH_COLLAPSED : SIDEBAR.WIDTH_EXPANDED;

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '100vh' }}>
        <LoadingSpinner label="Loading..." />
      </Box>
    );
  }

  if (!user) {
    return <Navigate to={ROUTES.LOGIN} state={{ from: location }} replace />;
  }

  return (
    <Box sx={{ display: 'flex', minHeight: '100vh' }}>
      <Sidebar />

      <Box
        component="main"
        sx={{
          flexGrow: 1,
          ml: `${sidebarWidth}px`,
          transition: `margin-left ${ANIMATION.NORMAL}ms ease`,
          p: 3,
          pt: 4,
        }}
      >
        <Container maxWidth="xl" disableGutters>
          <Outlet />
        </Container>
      </Box>
    </Box>
  );
}

export { AppLayout };
