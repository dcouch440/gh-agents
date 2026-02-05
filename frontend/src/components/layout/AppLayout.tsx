import Box from '@mui/material/Box';
import Container from '@mui/material/Container';
import { Outlet } from 'react-router-dom';
import { Sidebar } from './Sidebar';
import { useSidebar } from '@/hooks/useSidebar';
import { SIDEBAR, ANIMATION } from '@/constants';

function AppLayout() {
  const { collapsed } = useSidebar();
  const sidebarWidth = collapsed ? SIDEBAR.WIDTH_COLLAPSED : SIDEBAR.WIDTH_EXPANDED;

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
